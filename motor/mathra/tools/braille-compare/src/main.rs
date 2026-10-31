//! mathra against MathCAT on the same MathML (MathJax's), in Nemeth and in CMU.
//! usage: braille-compare <rows.json> <absolute rules dir> <out.json> (see README.md)

use serde_json::{json, Value};

fn dots(s: &str) -> String {
    s.chars()
        .map(|c| match c as u32 {
            0x2800 => "0".to_string(),
            u @ 0x2801..=0x28FF => (0..8).filter(|b| (u - 0x2800) >> b & 1 == 1).map(|b| char::from(b'1' + b as u8)).collect(),
            _ => c.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn mathcat_braille(mml: &str) -> Result<String, String> {
    libmathcat::set_mathml(mml).map_err(|e| libmathcat::errors_to_string(&e))?;
    libmathcat::get_braille("").map_err(|e| libmathcat::errors_to_string(&e))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rows: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    if let Err(e) = libmathcat::set_rules_dir(&args[2]) {
        eprintln!("rules: {}", libmathcat::errors_to_string(&e));
        std::process::exit(2);
    }
    eprintln!("MathCAT {}; braille codes {:?}", libmathcat::get_version(), libmathcat::get_supported_braille_codes());
    let mut out = Vec::new();
    for (code, mathra_code, locale, cat_lang) in [("Nemeth", mathra::BrailleCode::Nemeth, mathra::Locale::En, "en"), ("CMU", mathra::BrailleCode::Cmu, mathra::Locale::PtBr, "es")] {
        for (k, v) in [("Language", cat_lang), ("BrailleCode", code)] {
            if let Err(e) = libmathcat::set_preference(k, v) {
                eprintln!("{k}={v}: {}", libmathcat::errors_to_string(&e));
                std::process::exit(2);
            }
        }
        let (mut same, mut differ, mut skipped) = (0, 0, 0);
        for row in &rows {
            let Some(mml) = row["mml"].as_str() else { skipped += 1; continue };
            // MathJax writes commands it does not know in red: nothing to compare
            if mml.contains("mathcolor=\"red\"") {
                skipped += 1;
                continue;
            }
            let ours = mathra::convert(mml, &mathra::Options { input: mathra::Input::MathMl, locale, braille: Some(mathra_code), ..Default::default() });
            let ours = match ours {
                Ok(r) => r.braille,
                Err(e) => format!("ERROR {e}"),
            };
            let theirs = mathcat_braille(mml).unwrap_or_else(|e| format!("ERROR {e}"));
            // MathCAT uses U+2800 for spaces too; compare ignoring a trailing blank
            let equal = ours.trim_end_matches('⠀') == theirs.trim_end_matches('⠀');
            if equal { same += 1 } else { differ += 1 }
            out.push(json!({
                "code": code, "id": row["id"], "tex": row["tex"], "equal": equal,
                "mathra": ours, "mathcat": theirs, "mathra_dots": dots(&ours), "mathcat_dots": dots(&theirs),
            }));
        }
        eprintln!("{code}: {same} equal, {differ} different, {skipped} skipped");
    }
    std::fs::write(&args[3], serde_json::to_string_pretty(&out).unwrap()).unwrap();
}
