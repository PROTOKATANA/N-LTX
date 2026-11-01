use crate::keys;
use crate::tex::{self, Token};
use crate::Error;

use super::super::Engine;

/// The sets named by the options of the node being made (`set=a,set=b` appends to this macro).
pub(super) fn sets_macro(e: &mut Engine) -> String {
    if !e.tex.is_defined("tikra@lg@sets") {
        return String::new();
    }
    e.tex.expand_to_string(vec![Token::cs("tikra@lg@sets")]).unwrap_or_default()
}

pub(super) fn set_value(e: &mut Engine, v: &Option<Vec<Token>>) -> Result<String, Error> {
    Ok(match v {
        Some(v) => e.tex.expand_to_string(v.clone())?.trim().to_string(),
        None => String::new(),
    })
}

/// `set=name` in a node's options: the node joins the set once it has its name.
pub(super) fn k_set(e: &mut Engine, v: Option<Vec<Token>>) -> Result<(), Error> {
    let name = set_value(e, &v)?;
    if !e.node_sets.sets.contains_key(&name) {
        e.warn(format!("undefined set '{name}'"));
        return Ok(());
    }
    let mut sets = sets_macro(e);
    sets.push_str(&name);
    sets.push(',');
    e.tex.define_string("tikra@lg@sets", &sets, false);
    Ok(())
}

/// `/tikz/set`: any node can carry it, so it lives with the nodes, not with the graphs piece.
pub(crate) fn define_sets(e: &mut Engine) {
    let id = super::super::NATIVE_KEY_BASE + e.key_fns.len() as u16;
    e.key_fns.push(k_set);
    e.tex.define(&keys::macro_name("/tikz/set/.@cmd"), tex::Meaning::Native(id));
}
