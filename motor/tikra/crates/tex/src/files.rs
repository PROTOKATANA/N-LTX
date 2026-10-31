//! Files a figure reads: `filecontents` environments, and files under `TIKRA_INPUT_DIR`
//! (`:`-separated; the current directory if unset). Only relative names without `..`.

use std::path::PathBuf;

/// Largest file [`read_file`] reads.
pub const MAX_FILE: u64 = 4 << 20;

/// Takes the `filecontents` environments out of `code`, with the file (name, lines) each one
/// writes: the file has to keep its lines, which the tokens of a text do not.
pub fn split_filecontents(code: &str) -> (String, Vec<(String, String)>) {
    const BEGIN: &str = "\\begin{filecontents";
    let (mut rest, mut kept, mut files) = (code, String::new(), Vec::new());
    while let Some(at) = rest.find(BEGIN) {
        let Some((name, lines, after)) = filecontents_at(&rest[at + BEGIN.len()..]) else { break };
        kept.push_str(&rest[..at]);
        files.push((name, lines));
        rest = after;
    }
    kept.push_str(rest);
    (kept, files)
}

/// What comes after `\begin{filecontents`: `*}[options]{name}`, the lines and its `\end`.
fn filecontents_at(t: &str) -> Option<(String, String, &str)> {
    let (star, t) = t.strip_prefix('*').map_or(("", t), |r| ("*", r));
    let t = t.strip_prefix('}')?.trim_start();
    let t = match t.strip_prefix('[') {
        Some(r) => r.split_once(']')?.1.trim_start(),
        None => t,
    };
    let (name, t) = t.strip_prefix('{')?.split_once('}')?;
    let (lines, after) = t.split_once(&format!("\\end{{filecontents{star}}}"))?;
    Some((name.trim().to_string(), lines.to_string(), after))
}

/// Reads a table file, if it may be read: the folders searched and the reason it was refused.
pub fn read_file(name: &str, search: &[String]) -> Result<String, String> {
    let bytes = read_bytes(name, search, MAX_FILE)?;
    String::from_utf8(bytes).map_err(|e| format!("file '{}': {e}", name.trim()))
}

/// Reads a file of at most `max` bytes from the input folders (with the folders of `search` in
/// each), as [`read_file`] finds it.
pub fn read_bytes(name: &str, search: &[String], max: u64) -> Result<Vec<u8>, String> {
    let n = name.trim();
    let full = find_file(n, search)?;
    let meta = std::fs::metadata(&full).map_err(|e| format!("file '{n}': {e}"))?;
    if meta.len() > max {
        return Err(format!("file '{n}' is larger than {max} bytes"));
    }
    std::fs::read(&full).map_err(|e| format!("file '{n}': {e}"))
}

/// Where a file is: the first input folder (and folder of `search` in it) that has it.
pub fn find_file(name: &str, search: &[String]) -> Result<PathBuf, String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("no file name".into());
    }
    let p = std::path::Path::new(n);
    if p.is_absolute() || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(format!("file '{n}' refused: only relative names inside the input folders are read"));
    }
    let roots: Vec<PathBuf> = match std::env::var("TIKRA_INPUT_DIR") {
        Ok(v) if !v.is_empty() => v.split(':').map(PathBuf::from).collect(),
        _ => vec![PathBuf::from(".")],
    };
    for root in &roots {
        for dir in search.iter().map(|s| s.trim()).chain(std::iter::once(".")) {
            if std::path::Path::new(dir).components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                continue;
            }
            let full = root.join(dir).join(n);
            if std::fs::metadata(&full).is_ok_and(|m| m.is_file()) {
                return Ok(full);
            }
        }
    }
    Err(format!("file '{n}' not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filecontents_leaves_the_text_and_keeps_the_lines() {
        let (kept, files) = split_filecontents("a\\begin{filecontents*}[overwrite]{x.dat}\n1 2\n3 4\n\\end{filecontents*}b");
        assert_eq!(kept, "ab");
        assert_eq!(files, [("x.dat".to_string(), "\n1 2\n3 4\n".to_string())]);
        let (kept, files) = split_filecontents("\\begin{filecontents}{y}\n1\n\\end{filecontents}\\begin{filecontents}{z}");
        assert_eq!(kept, "\\begin{filecontents}{z}", "the one that does not end stays");
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn refuses_escapes() {
        assert!(read_file("../secret", &[]).is_err());
        assert!(read_file("/etc/passwd", &[]).is_err());
    }
}
