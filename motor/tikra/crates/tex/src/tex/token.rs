//! Tokens, category codes, primitives and what a control sequence means.

use std::rc::Rc;

/// Category codes, as in TeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cat {
    Escape,
    Begin,
    End,
    Math,
    Align,
    EndLine,
    Param,
    Sup,
    Sub,
    Ignored,
    Space,
    Letter,
    Other,
    Active,
    Comment,
    Invalid,
}

impl Cat {
    pub fn from_code(n: i64) -> Option<Cat> {
        use Cat::*;
        Some(match n {
            0 => Escape,
            1 => Begin,
            2 => End,
            3 => Math,
            4 => Align,
            5 => EndLine,
            6 => Param,
            7 => Sup,
            8 => Sub,
            9 => Ignored,
            10 => Space,
            11 => Letter,
            12 => Other,
            13 => Active,
            14 => Comment,
            15 => Invalid,
            _ => return None,
        })
    }

    pub fn code(self) -> i64 {
        self as i64
    }
}

/// A token: a control sequence, or a character with its category.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Cs(Rc<str>),
    Char(char, Cat),
}

impl Token {
    pub fn cs(name: &str) -> Token {
        Token::Cs(Rc::from(name))
    }
    pub fn other(c: char) -> Token {
        Token::Char(c, Cat::Other)
    }
    pub fn is_char(&self, c: char) -> bool {
        matches!(self, Token::Char(x, cat) if *x == c && !matches!(cat, Cat::Active))
    }
    pub fn is_cs(&self, name: &str) -> bool {
        matches!(self, Token::Cs(n) if &**n == name)
    }
    pub fn is_space(&self) -> bool {
        matches!(self, Token::Char(_, Cat::Space))
    }
}

/// A parameter text item: a token to match, or `#n`.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Tok(Token),
    Arg(u8),
}

/// A body item: a token, or `#n` to replace.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Tok(Token),
    Arg(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Macro {
    pub params: Vec<Param>,
    pub body: Vec<Body>,
    /// `\newcommand\x[2][default]{…}`: the first argument is optional, in brackets.
    pub optional: Option<Vec<Token>>,
    pub protected: bool,
}

/// TeX primitives (and the few LaTeX and plain commands better done natively).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prim {
    // expandable
    Expandafter,
    Noexpand,
    Csname,
    Endcsname,
    Stringify,
    Number,
    Romannumeral,
    The,
    ShowMeaning,
    Unexpanded,
    Detokenize,
    If,
    Ifcat,
    Ifx,
    Ifnum,
    Ifdim,
    Ifodd,
    Ifcase,
    Iftrue,
    Iffalse,
    Ifdefined,
    Ifcsname,
    Ifvmode,
    Ifhmode,
    Ifmmode,
    Ifinner,
    Else,
    Or,
    Fi,
    Unless,
    Jobname,
    Pdfstrcmp,
    // unexpandable
    Relax,
    Def,
    Gdef,
    Edef,
    Xdef,
    Let,
    Futurelet,
    Global,
    Long,
    Outer,
    Protected,
    Begingroup,
    Endgroup,
    Count,
    Dimen,
    Skip,
    Toks,
    Countdef,
    Dimendef,
    Skipdef,
    Toksdef,
    Chardef,
    Mathchardef,
    Advance,
    Multiply,
    Divide,
    Catcode,
    Numexpr,
    Dimexpr,
    Glueexpr,
    Afterassignment,
    Aftergroup,
    Ignorespaces,
    Par,
    Message,
    Immediate,
    Write,
    Typeout,
    Errmessage,
    Showtokens,
    Newcount,
    Newdimen,
    Newskip,
    Newlength,
    Newtoks,
    Newif,
    Setlength,
    Addtolength,
    Setcounter,
    Addtocounter,
    Newcounter,
    Stepcounter,
    Value,
    Arabic,
    Newcommand,
    Renewcommand,
    Providecommand,
    Declarerobustcommand,
    /// `\NewDocumentCommand` and its kin (the kind by the name used)
    Newdocumentcommand,
    /// what a document command expands to first: reads its arguments by its specification
    Documentarguments,
    /// `\IfNoValueTF`, `\IfBooleanTF` and their kin (the kind by the name used)
    Documentif,
    Newenvironment,
    Renewenvironment,
    Makeatletter,
    Makeatother,
    Ifnextchar,
    Firstoftwo,
    Secondoftwo,
    Gobble,
    Gobbletwo,
    Firstofone,
    Ifthenelse,
    Whiledo,
    Ifstrequal,
    /// etoolbox's `\ifstrempty`, `\ifblank`, `\ifdefempty` (the kind by the name used)
    Ifstrempty,
    Ifboolean,
    Newboolean,
    /// `\g@addto@macro`, etoolbox's `\appto` and its kin, babel's `\addto` (the kind by the name used)
    Addtomacro,
    /// the xstring package's commands (by the name used)
    Xstring,
    Setboolean,
    Wd,
    Ht,
    Dp,
    Setbox,
    Hbox,
    Uppercase,
    Lowercase,
    Show,
    Showthe,
}

/// What a control sequence (or active character) means.
#[derive(Debug, Clone, PartialEq)]
pub enum Meaning {
    Macro(Rc<Macro>),
    /// `\let\x=a`: the character itself.
    Char(char, Cat),
    Prim(Prim),
    Count(u32),
    Dimen(u32),
    Skip(u32),
    Toks(u32),
    CharDef(i64),
    /// A command of the drawing layer, by number.
    Native(u16),
    /// `\let\x\undefined`
    Undefined,
}

impl Meaning {
    pub fn is_expandable(&self) -> bool {
        match self {
            Meaning::Macro(_) => true,
            Meaning::Prim(p) => p.is_expandable(),
            _ => false,
        }
    }
}

impl Prim {
    pub fn is_expandable(self) -> bool {
        use Prim::*;
        matches!(
            self,
            Expandafter
                | Noexpand
                | Csname
                | Stringify
                | Number
                | Romannumeral
                | The
                | ShowMeaning
                | Unexpanded
                | Detokenize
                | If
                | Ifcat
                | Ifx
                | Ifnum
                | Ifdim
                | Ifodd
                | Ifcase
                | Iftrue
                | Iffalse
                | Ifdefined
                | Ifcsname
                | Ifvmode
                | Ifhmode
                | Ifmmode
                | Ifinner
                | Else
                | Or
                | Fi
                | Unless
                | Jobname
                | Pdfstrcmp
                | Ifnextchar
                | Firstoftwo
                | Secondoftwo
                | Gobble
                | Gobbletwo
                | Firstofone
                | Value
                | Arabic
                | Ifthenelse
                | Whiledo
                | Ifstrequal
                | Ifstrempty
                | Ifboolean
                | Documentarguments
                | Documentif
        )
    }

    pub(super) fn is_conditional(self) -> bool {
        use Prim::*;
        matches!(self, If | Ifcat | Ifx | Ifnum | Ifdim | Ifodd | Ifcase | Iftrue | Iffalse | Ifdefined | Ifcsname | Ifvmode | Ifhmode | Ifmmode | Ifinner)
    }
}
