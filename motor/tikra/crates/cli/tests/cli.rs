//! The command line: arguments, input from a file and from stdin, SVG and PNG output, errors.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const FIGURE: &str = r"\begin{tikzpicture} \draw[->] (0,0) -- (1,1) node[right] {$x$}; \end{tikzpicture}";
const WITH_WARNING: &str = r"\begin{tikzpicture} \draw (0,0) -- (1,1); \matrix {a}; \end{tikzpicture}";

fn tikra(args: &[&str], stdin: Option<&[u8]>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tikra"));
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("the binary runs");
    {
        let mut input = child.stdin.take().unwrap();
        if let Some(b) = stdin {
            // a binary that stops before reading (an unknown option) closes its input: not an error
            if let Err(e) = input.write_all(b) {
                assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe, "{e}");
            }
        }
    }
    child.wait_with_output().unwrap()
}

/// A temporary directory for this test only.
fn folder(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("tikra-cli-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn help_and_version() {
    for a in ["-h", "--help"] {
        let o = tikra(&[a], None);
        assert!(o.status.success());
        assert!(text(&o.stdout).contains("USAGE"), "{a}");
    }
    for a in ["-V", "--version"] {
        let o = tikra(&[a], None);
        assert!(o.status.success());
        assert!(text(&o.stdout).starts_with("tikra "), "{a}");
    }
}

#[test]
fn stdin_to_stdout() {
    let o = tikra(&[], Some(FIGURE.as_bytes()));
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(text(&o.stdout).starts_with("<svg"));
}

#[test]
fn file_to_svg_and_png() {
    let d = folder("outputs");
    let tex = d.join("f.tex");
    std::fs::write(&tex, FIGURE).unwrap();
    let svg = d.join("f.svg");
    let o = tikra(&[tex.to_str().unwrap(), "-o", svg.to_str().unwrap()], None);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert!(std::fs::read_to_string(&svg).unwrap().starts_with("<svg"));
    let png = d.join("f.png");
    let o = tikra(&[tex.to_str().unwrap(), "--output", png.to_str().unwrap(), "--dpi", "72", "--transparent"], None);
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(&std::fs::read(&png).unwrap()[..4], b"\x89PNG");
    // a dpi that is not a number: back to the default
    let o = tikra(&[tex.to_str().unwrap(), "-o", png.to_str().unwrap(), "--dpi", "lots"], None);
    assert!(o.status.success());
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn warnings_on_stderr_or_listed() {
    let o = tikra(&[], Some(WITH_WARNING.as_bytes()));
    assert!(o.status.success());
    assert!(text(&o.stderr).contains("tikra: warning:"));
    assert!(text(&o.stdout).starts_with("<svg"));
    // --warnings lists them on stdout and does not print the SVG
    let o = tikra(&["--warnings"], Some(WITH_WARNING.as_bytes()));
    assert!(o.status.success());
    assert!(text(&o.stdout).starts_with("warning: "));
    assert!(!text(&o.stdout).contains("<svg"));
}

#[test]
fn the_figure_in_words() {
    // --describe prints the alt text and the description, not the SVG
    let o = tikra(&["--describe"], Some(FIGURE.as_bytes()));
    assert!(o.status.success(), "{}", text(&o.stderr));
    assert_eq!(text(&o.stdout), "alt: Drawing of one shape with the text “x”\ndescription: Text in the figure: x.\n");
    // in another language; the SVG is drawn the same
    let o = tikra(&["--describe", "--locale", "pt-BR"], Some(FIGURE.as_bytes()));
    assert_eq!(text(&o.stdout), "alt: Desenho de uma forma com o texto “x”\ndescription: Texto na figura: x.\n");
    let o = tikra(&["-l", "klingon"], Some(FIGURE.as_bytes()));
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o.stderr).contains("unknown locale: klingon"));
}

#[test]
fn input_errors() {
    let o = tikra(&["/path/that/does/not/exist.tex"], None);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("could not read"));
    // a file that is not UTF-8
    let d = folder("binary");
    let bin = d.join("b.tex");
    std::fs::write(&bin, [0xff, 0xfe, 0x00, 0x5c]).unwrap();
    let o = tikra(&[bin.to_str().unwrap()], None);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("could not read"));
    std::fs::remove_dir_all(&d).unwrap();
    // stdin that is not UTF-8
    let o = tikra(&[], Some(&[0xc3, 0x28]));
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("standard input"));
    // no figure
    let o = tikra(&[], Some(b"nothing here"));
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("no tikzpicture"));
}

#[test]
fn output_errors() {
    let o = tikra(&["-o", "/path/that/does/not/exist/f.svg"], Some(FIGURE.as_bytes()));
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("could not write"));
    let o = tikra(&["-o", "/path/that/does/not/exist/f.png"], Some(FIGURE.as_bytes()));
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("could not write"));
    // a PNG past the pixel cap
    let o = tikra(&["-o", "/tmp/will-not-be-written.png", "--dpi", "1000000"], Some(FIGURE.as_bytes()));
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o.stderr).contains("too large"));
}
