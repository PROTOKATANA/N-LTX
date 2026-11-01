//! Scanning numbers, dimensions and expressions.

use super::*;
use crate::Error;

impl Tex {
    // ---------------------------------------------------------------- scanning numbers and lengths

    pub(super) fn scan_register_number(&mut self) -> Result<u32, Error> {
        let n = self.scan_int()?;
        if !(0..=65535).contains(&n) {
            return Err(Error::Figure(format!("bad register number {n}")));
        }
        Ok(n as u32)
    }

    /// An integer, as TeX reads one: signs, digits, `"hex`, `'octal`, `` `c ``, or an internal integer.
    pub fn scan_int(&mut self) -> Result<i64, Error> {
        let mut sign = 1i64;
        loop {
            let t = self.next_x_nonspace()?.ok_or_else(|| Error::Figure("missing number".into()))?;
            if t.is_char('-') {
                sign = -sign;
                continue;
            }
            if t.is_char('+') {
                continue;
            }
            if let Some(v) = self.internal_int(&t)? {
                return Ok(sign * v);
            }
            if let Token::Char(c, _) = t {
                if c == '`' {
                    let x = self.get_token().ok_or_else(|| Error::Figure("missing character after `".into()))?;
                    let v = match &x {
                        Token::Char(c, _) => *c as i64,
                        Token::Cs(n) => n.chars().next().map_or(0, |c| c as i64),
                    };
                    self.skip_one_optional_space()?;
                    return Ok(sign * v);
                }
                let radix = match c {
                    '"' => 16,
                    '\'' => 8,
                    _ => 10,
                };
                let mut digits = String::new();
                if radix == 10 {
                    if !c.is_ascii_digit() {
                        self.back(t);
                        return Err(Error::Figure("missing number".into()));
                    }
                    digits.push(c);
                }
                while let Some(x) = self.get_token() {
                    // `\ifcase2\or …`: an `\or`, `\else` or `\fi` ends the number, as TeX does it
                    // when it meets one while the condition is still incomplete
                    let m = self.meaning(&x);
                    if matches!(m, Meaning::Prim(Prim::Or | Prim::Else | Prim::Fi)) {
                        self.back(x);
                        break;
                    }
                    if m.is_expandable() {
                        self.expand(&x, &m)?;
                        continue;
                    }
                    match x {
                        Token::Char(d, Cat::Other | Cat::Letter) if d.is_digit(radix) && (radix == 16 && d.is_ascii_uppercase() || d.is_ascii_digit()) => digits.push(d),
                        Token::Char(_, Cat::Space) => break,
                        other => {
                            self.back(other);
                            break;
                        }
                    }
                }
                let v = i64::from_str_radix(&digits, radix).unwrap_or(2_147_483_647).min(2_147_483_647);
                return Ok(sign * v);
            }
            self.back(t);
            return Err(Error::Figure("missing number".into()));
        }
    }

    fn skip_one_optional_space(&mut self) -> Result<(), Error> {
        if let Some(t) = self.get_x_token()? {
            if !t.is_space() {
                self.back(t);
            }
        }
        Ok(())
    }

    /// The value of an internal integer quantity, if `t` is one.
    fn internal_int(&mut self, t: &Token) -> Result<Option<i64>, Error> {
        Ok(match self.meaning(t) {
            Meaning::Count(r) => Some(self.counts.get(&r).copied().unwrap_or(0)),
            Meaning::CharDef(v) => Some(v),
            Meaning::Dimen(r) | Meaning::Skip(r) => Some(self.dimens.get(&r).copied().unwrap_or(0)),
            Meaning::Prim(Prim::Count) => {
                let r = self.scan_register_number()?;
                Some(self.counts.get(&r).copied().unwrap_or(0))
            }
            Meaning::Prim(Prim::Dimen | Prim::Skip) => {
                let r = self.scan_register_number()?;
                Some(self.dimens.get(&r).copied().unwrap_or(0))
            }
            Meaning::Prim(Prim::Numexpr) => Some(self.scan_numexpr()?),
            Meaning::Prim(Prim::Dimexpr | Prim::Glueexpr) => Some(self.scan_dimexpr()?),
            Meaning::Prim(Prim::Catcode) => {
                let c = self.scan_int()?;
                Some(self.catcode(char::from_u32(c.max(0) as u32).unwrap_or(' ')).code())
            }
            Meaning::Prim(p @ (Prim::Wd | Prim::Ht | Prim::Dp)) => {
                let n = self.scan_int()?;
                Some(self.box_dimen(p, n))
            }
            _ => None,
        })
    }

    /// `\wd`, `\ht` or `\dp` of box register `n` (sp).
    pub(super) fn box_dimen(&self, p: Prim, n: i64) -> i64 {
        let k = match p {
            Prim::Wd => 0,
            Prim::Ht => 1,
            _ => 2,
        };
        self.box_dims.get(&n).map_or(0, |d| d[k])
    }

    fn internal_dimen(&mut self, t: &Token) -> Result<Option<i64>, Error> {
        Ok(match self.meaning(t) {
            Meaning::Dimen(r) | Meaning::Skip(r) => Some(self.dimens.get(&r).copied().unwrap_or(0)),
            Meaning::Prim(Prim::Dimen | Prim::Skip) => {
                let r = self.scan_register_number()?;
                Some(self.dimens.get(&r).copied().unwrap_or(0))
            }
            Meaning::Prim(Prim::Dimexpr | Prim::Glueexpr) => Some(self.scan_dimexpr()?),
            Meaning::Prim(p @ (Prim::Wd | Prim::Ht | Prim::Dp)) => {
                let n = self.scan_int()?;
                Some(self.box_dimen(p, n))
            }
            _ => None,
        })
    }

    /// A dimension, as TeX reads one: a factor (possibly with decimals) and a unit, or an internal dimension.
    pub fn scan_dimen(&mut self) -> Result<i64, Error> {
        let mut sign = 1i64;
        loop {
            let t = self.next_x_nonspace()?.ok_or_else(|| Error::Figure("missing dimension".into()))?;
            if t.is_char('-') {
                sign = -sign;
                continue;
            }
            if t.is_char('+') {
                continue;
            }
            if let Some(v) = self.internal_dimen(&t)? {
                return Ok(sign * v);
            }
            // a factor: an internal integer, or digits with a decimal point or comma
            let (integer, digits): (i64, Vec<u8>) = if let Some(v) = self.internal_int(&t)? {
                (v, Vec::new())
            } else {
                let mut int = String::new();
                let mut frac = Vec::new();
                let mut point = false;
                let mut x = Some(t);
                while let Some(tok) = x.take() {
                    match tok {
                        Token::Char(d, _) if d.is_ascii_digit() => {
                            if point {
                                frac.push(d as u8 - b'0');
                            } else {
                                int.push(d);
                            }
                            x = self.get_x_token()?;
                        }
                        Token::Char('.' | ',', _) if !point => {
                            point = true;
                            x = self.get_x_token()?;
                        }
                        other => self.back(other),
                    }
                }
                if int.is_empty() && !point {
                    return Err(Error::Figure("missing dimension".into()));
                }
                (int.parse().unwrap_or(0).min(2_147_483_647), frac)
            };
            // the unit: an internal dimension multiplies, or a keyword
            let u = self.next_x_nonspace()?;
            if let Some(u) = u {
                if let Some(v) = self.internal_dimen(&u)? {
                    let (frac, _) = dimen::xn_over_d(v, dimen::round_decimals(&digits), 65536);
                    return Ok(sign * (dimen::mult(integer, v) + frac));
                }
                self.back(u);
            }
            self.scan_keyword("true")?;
            for (name, _) in dimen::UNITS {
                if self.scan_keyword(name)? {
                    self.skip_one_optional_space()?;
                    let v = dimen::exact(integer, &digits, name).unwrap_or(0);
                    return Ok(sign * v.clamp(-dimen::MAX_DIMEN, dimen::MAX_DIMEN));
                }
            }
            return Err(Error::Figure("illegal unit of measure (pt inserted)".into()));
        }
    }

    /// Reads `keyword` (letters, any case) if it comes next; otherwise leaves the input as it was.
    pub fn scan_keyword(&mut self, keyword: &str) -> Result<bool, Error> {
        let mut read = Vec::new();
        let mut first = true;
        for k in keyword.chars() {
            let t = if first { self.next_x_nonspace()? } else { self.get_x_token()? };
            first = false;
            match t {
                Some(Token::Char(c, _)) if c.eq_ignore_ascii_case(&k) => read.push(Token::Char(c, Cat::Letter)),
                Some(other) => {
                    self.back(other);
                    self.push_tokens(read);
                    return Ok(false);
                }
                None => {
                    self.push_tokens(read);
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// `plus 1fil minus 2pt` after a skip value: read and ignored (no stretching in figures).
    pub(super) fn skip_glue_parts(&mut self) -> Result<(), Error> {
        for k in ["plus", "minus"] {
            if self.scan_keyword(k)? {
                let _ = self.scan_dimen_or_fil();
            }
        }
        Ok(())
    }

    fn scan_dimen_or_fil(&mut self) -> Result<(), Error> {
        let mut sign_and_number = Vec::new();
        loop {
            match self.get_x_token()? {
                Some(Token::Char(d, _)) if d.is_ascii_digit() || matches!(d, '.' | ',' | '-' | '+' | ' ') => sign_and_number.push(Token::other(d)),
                Some(other) => {
                    self.back(other);
                    break;
                }
                None => break,
            }
        }
        if self.scan_keyword("fil")? {
            while self.scan_keyword("l")? {}
            return Ok(());
        }
        self.push_tokens(sign_and_number);
        self.scan_dimen().map(|_| ())
    }

    pub(super) fn scan_optional_equals(&mut self) -> Result<(), Error> {
        if let Some(t) = self.next_x_nonspace()? {
            if !t.is_char('=') {
                self.back(t);
            }
        }
        Ok(())
    }

    /// `\numexpr …` up to `\relax` or a token that cannot continue the expression.
    pub(super) fn scan_numexpr(&mut self) -> Result<i64, Error> {
        let v = self.numexpr_sum(false)?;
        // an ending \relax is consumed
        if let Some(t) = self.next_x_nonspace()? {
            if !t.is_cs("relax") {
                self.back(t);
            }
        }
        Ok(v)
    }

    fn numexpr_sum(&mut self, dim: bool) -> Result<i64, Error> {
        let mut v = self.numexpr_term(dim)?;
        loop {
            let Some(t) = self.next_x_nonspace()? else { return Ok(v) };
            if t.is_char('+') {
                v += self.numexpr_term(dim)?;
            } else if t.is_char('-') {
                v -= self.numexpr_term(dim)?;
            } else {
                self.back(t);
                return Ok(v);
            }
        }
    }

    fn numexpr_term(&mut self, dim: bool) -> Result<i64, Error> {
        let mut v = self.numexpr_factor(dim)?;
        loop {
            let Some(t) = self.next_x_nonspace()? else { return Ok(v) };
            if t.is_char('*') {
                let f = self.numexpr_factor(false)?;
                v = v.saturating_mul(f);
            } else if t.is_char('/') {
                let f = self.numexpr_factor(false)?;
                if f == 0 {
                    return Err(Error::Figure("division by 0 in \\numexpr".into()));
                }
                // e-TeX rounds the quotient
                v = ((v as f64) / (f as f64)).round() as i64;
            } else {
                self.back(t);
                return Ok(v);
            }
        }
    }

    fn numexpr_factor(&mut self, dim: bool) -> Result<i64, Error> {
        let t = self.next_x_nonspace()?.ok_or_else(|| Error::Figure("missing number in \\numexpr".into()))?;
        if t.is_char('(') {
            let v = self.numexpr_sum(dim)?;
            if let Some(c) = self.next_x_nonspace()? {
                if !c.is_char(')') {
                    self.back(c);
                }
            }
            return Ok(v);
        }
        if t.is_char('-') {
            return Ok(-self.numexpr_factor(dim)?);
        }
        self.back(t);
        if dim {
            self.scan_dimen()
        } else {
            self.scan_int()
        }
    }

    pub(super) fn scan_dimexpr(&mut self) -> Result<i64, Error> {
        let v = self.numexpr_sum(true)?;
        if let Some(t) = self.next_x_nonspace()? {
            if !t.is_cs("relax") {
                self.back(t);
            }
        }
        Ok(v)
    }
}
