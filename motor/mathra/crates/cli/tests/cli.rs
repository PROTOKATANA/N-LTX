//! The command line run as a process, the way someone would use it in a terminal.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

fn mathra(args: &[&str], input: Option<&[u8]>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mathra"));
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("the binary runs");
    {
        let mut stdin = child.stdin.take().unwrap();
        if let Some(e) = input {
            stdin.write_all(e).unwrap();
        }
    }
    child.wait_with_output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn folder(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("mathra-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn help_and_version() {
    for a in ["-h", "--help"] {
        let o = mathra(&[a], None);
        assert!(o.status.success());
        assert!(stdout(&o).contains("USAGE"));
    }
    for a in ["-V", "--version"] {
        let o = mathra(&[a], None);
        assert!(o.status.success());
        assert!(stdout(&o).starts_with("mathra "));
    }
}

#[test]
fn argument_errors_exit_with_code_2() {
    for (args, snippet) in [
        (&["x", "-o"][..], "--output"),
        (&["x", "--locale"][..], "--locale"),
        (&["x", "-l", "klingon"][..], "klingon"),
        (&["x", "--made-up"][..], "unknown option"),
        (&["x", "y"][..], "single equation"),
    ] {
        let o = mathra(args, None);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(stderr(&o).contains(snippet), "{args:?}: {}", stderr(&o));
    }
}

#[test]
fn no_equation_and_no_stdin_shows_help() {
    let o = mathra(&[], Some(b"   \n"));
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("USAGE"));
}

#[test]
fn equation_from_stdin() {
    let o = mathra(&[], Some(b"  x^2  \n"));
    assert!(o.status.success());
    assert!(stdout(&o).starts_with("<svg"));
}

#[test]
fn output_modes() {
    let o = mathra(&["\\frac{a}{b}", "--tree"], None);
    assert!(stdout(&o).trim_start().starts_with('{'));
    let o = mathra(&["\\frac{a}{b}", "--speech"], None);
    assert!(stdout(&o).contains("over"), "{}", stdout(&o));
    let o = mathra(&["\\frac{a}{b}", "--speech", "--locale", "pt-BR"], None);
    assert!(stdout(&o).contains("sobre"), "{}", stdout(&o));
    let o = mathra(&["\\frac{a}{b}", "--text"], None);
    assert!(stdout(&o).contains("a/b"), "{}", stdout(&o));
    let o = mathra(&["x", "--inline", "-l", "en"], None);
    assert!(stdout(&o).starts_with("<svg"));
}

#[test]
fn explain_lists_rules_and_warnings() {
    let o = mathra(&["3,14", "--explain", "--locale", "pt-BR"], None);
    let s = stdout(&o);
    assert!(s.contains("rules\n") && s.contains("L-NUM"), "{s}");
    let o = mathra(&["x", "--explain"], None);
    assert!(stdout(&o).contains("no rule changed"));
    let o = mathra(&["\\madeupcommand x", "--explain"], None);
    let s = stdout(&o);
    assert!(s.contains("warnings\n"), "{s}");
    assert!(stderr(&o).is_empty(), "with --explain the warning goes to stdout, not stderr");
}

#[test]
fn warning_goes_to_stderr_and_strict_refuses() {
    let o = mathra(&["\\madeupcommand x"], None);
    assert!(o.status.success());
    assert!(stderr(&o).contains("warning"));
    let o = mathra(&["\\madeupcommand x", "--strict"], None);
    assert_eq!(o.status.code(), Some(1));
    assert!(!stderr(&o).is_empty());
}

#[test]
fn writes_to_file() {
    let p = folder("file");
    let file = p.join("x.svg");
    let o = mathra(&["x", "-o", file.to_str().unwrap()], None);
    assert!(o.status.success());
    assert!(std::fs::read_to_string(&file).unwrap().starts_with("<svg"));
    let o = mathra(&["x", "--output", p.join("does/not/exist/x.svg").to_str().unwrap()], None);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("could not write"));
}

#[test]
fn batch_one_line_per_equation() {
    let p = folder("batch");
    let o = mathra(&["--batch", "--output", p.to_str().unwrap()], Some(b"x^2\n\\madeupcommand\n"));
    assert!(o.status.success());
    let lines: Vec<String> = stdout(&o).lines().map(str::to_string).collect();
    assert_eq!(lines[0], "ok");
    assert!(lines[1].starts_with("warning\t"), "{lines:?}");
    assert!(p.join("1.svg").exists() && p.join("2.svg").exists());

    let o = mathra(&["--batch", "--strict"], Some(b"\\frac{1}\nx\n"));
    let lines: Vec<String> = stdout(&o).lines().map(str::to_string).collect();
    assert!(lines[0].starts_with("error\t"), "{lines:?}");
    assert_eq!(lines[1], "ok");
}

#[test]
fn batch_flags_non_utf8_line_and_continues() {
    let o = mathra(&["--batch"], Some(b"x\n\xff\xfe\ny\n"));
    assert!(o.status.success());
    assert_eq!(stdout(&o), "ok\nerror\tthe line is not UTF-8\nok\n");
}

// --- problem cases ---

#[test]
fn batch_with_missing_folder_errors_on_every_line() {
    let p = folder("no-folder").join("does-not-exist");
    let o = mathra(&["--batch", "-o", p.to_str().unwrap()], Some(b"x\ny\n"));
    let s = stdout(&o);
    assert_eq!(s.lines().count(), 2);
    assert!(s.lines().all(|l| l.starts_with("error\tcould not write")), "{s}");
}

#[test]
fn batch_accepts_crlf_and_empty_lines_keeping_numbering() {
    let p = folder("crlf");
    let o = mathra(&["--batch", "-o", p.to_str().unwrap()], Some(b"x\r\n\n\\frac{1}{2}\r\n"));
    assert_eq!(stdout(&o), "ok\nok\nok\n");
    assert!(p.join("3.svg").exists(), "the 3rd line is still 3.svg");
}

#[test]
fn empty_equation_gives_valid_empty_svg() {
    let o = mathra(&[""], None);
    assert!(o.status.success());
    assert!(stdout(&o).contains(r#"viewBox="0 0 0 0""#));
}

#[test]
fn output_to_a_folder_is_a_clear_error() {
    let p = folder("folder-as-file");
    let o = mathra(&["x", "-o", p.to_str().unwrap()], None);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("could not write"));
}

#[test]
fn huge_argument() {
    let tex = "x+".repeat(50_000) + "x";
    let o = mathra(&[&tex, "--text"], None);
    assert!(o.status.success());
    assert_eq!(stdout(&o).trim().chars().filter(|c| *c == '+').count(), 50_000);
}

#[test]
fn input_formats() {
    let o = mathra(&["sum_(i=1)^n i", "--from", "asciimath", "--text"], None);
    assert!(o.status.success(), "{}", stderr(&o));
    assert_eq!(stdout(&o).trim(), "∑_(i = 1)^n i");
    let o = mathra(&["-f", "mathml", "--speech"], Some(b"<math><msqrt><mi>x</mi></msqrt></math>"));
    assert_eq!(stdout(&o).trim(), "square root of x");
    // MathML passed as LaTeX is recognized
    let o = mathra(&["<math><mn>1</mn><mo>+</mo><mn>2</mn></math>", "--explain"], None);
    assert!(stdout(&o).contains("L-ENT-4"), "{}", stdout(&o));
    let o = mathra(&["x", "--from", "word"], None);
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("unknown input format"));
    assert!(stdout(&mathra(&["--help"], None)).contains("--from"));
}

#[test]
fn braille_output() {
    let o = mathra(&[r"\frac{3}{4}", "--locale", "pt-BR", "--braille"], None);
    assert_eq!(stdout(&o).trim(), "⠼⠒⠙");
    let o = mathra(&[r"\frac{3}{4}", "--braille", "--braille-code", "nemeth"], None);
    assert_eq!(stdout(&o).trim(), "⠹⠒⠌⠲⠼");
    let o = mathra(&["x^2", "--explain"], None);
    assert!(stdout(&o).contains("braille  ⠭⠘⠆ (nemeth)"), "{}", stdout(&o));
    let o = mathra(&["x", "--braille-code", "ueb"], None);
    assert_eq!(o.status.code(), Some(2));
    assert!(stderr(&o).contains("unknown braille code"));
}

#[test]
fn animate_and_steps() {
    let plain = stdout(&mathra(&[r"\frac{a}{b}"], None));
    let steps = stdout(&mathra(&[r"\frac{a}{b}", "--steps"], None));
    let animated = stdout(&mathra(&[r"\frac{a}{b}", "--animate"], None));
    assert!(!plain.contains("data-step"));
    assert!(steps.contains(r#"data-steps="3""#) && !steps.contains("@keyframes"));
    assert!(animated.contains(r#"data-steps="3""#) && animated.contains("@keyframes mr-f") && animated.contains("--mr-step"));
    assert!(stdout(&mathra(&["--help"], None)).contains("--animate"));
}
