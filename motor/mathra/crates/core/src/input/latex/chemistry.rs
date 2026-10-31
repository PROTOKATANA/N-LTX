//! L-CHE-2: the chemfig constructions inside an equation. The reader takes the whole construction as
//! written (`\chemfig{…}`, `\schemestart … \schemestop`, `\chemname{…}{…}`, and the `\chemmove{…}`
//! right after any of them) and hands it to the chemistry reader, which returns one node.

use super::*;
use crate::chem;

impl Reader<'_> {
    /// `name` was just read; the reader is right after it.
    pub(super) fn chem_construction(&mut self, name: &str, col: usize) -> R<Node> {
        let mut source = format!("\\{name}");
        match name {
            "schemestart" => {
                let stop: Vec<char> = "\\schemestop".chars().collect();
                let start = self.pos;
                match (start..self.c.len()).find(|&i| self.c[i..].starts_with(&stop)) {
                    Some(end) => {
                        source.extend(&self.c[start..end]);
                        self.pos = end + stop.len();
                    }
                    None => {
                        self.tolerate(rules::E_REC_5, col, "\\schemestart without \\schemestop", "closed at the end")?;
                        source.extend(&self.c[start..]);
                        self.pos = self.c.len();
                    }
                }
            }
            _ => {
                // \chemfig[options]{body}, \chemname[dim]{compound}{name}, \chemname*{…}{…}
                if name == "chemname" && self.current() == Some('*') {
                    self.pos += 1;
                }
                if let Some(options) = self.raw_optional() {
                    source.push_str(&format!("[{options}]"));
                }
                for _ in 0..if name == "chemname" { 2 } else { 1 } {
                    let arg = self.raw_text()?;
                    source.push_str(&format!("{{{arg}}}"));
                }
            }
        }
        // the mechanism arrows that follow belong to it
        loop {
            let saved = self.pos;
            self.skip_spaces();
            let next: String = self.c[self.pos..].iter().take(9).collect();
            if next != "\\chemmove" || self.c.get(self.pos + 9).is_some_and(char::is_ascii_alphabetic) {
                self.pos = saved;
                break;
            }
            self.pos += 9;
            let options = self.raw_optional().map(|o| format!("[{o}]")).unwrap_or_default();
            let body = self.raw_text()?;
            source.push_str(&format!(" \\chemmove{options}{{{body}}}"));
        }
        let mut reader = chem::scheme::Reader { strict: self.strict, registry: self.registry, column: col };
        let scheme = chem::scheme::read(&source, &mut reader)?;
        let formulas: Vec<String> = scheme
            .items
            .iter()
            .filter_map(|i| match i {
                chem::model::Item::Compound(chem::model::Compound { body: chem::model::Body::Structure(m), .. }) => Some(chem::formula::of(m).written()),
                _ => None,
            })
            .collect();
        let what = if formulas.is_empty() { "reaction scheme".to_string() } else { formulas.join(", ") };
        self.registry.note(rules::L_CHE_2, format!("\\{name}: {what}"));
        Ok(Node::Chemistry { scheme })
    }
}
