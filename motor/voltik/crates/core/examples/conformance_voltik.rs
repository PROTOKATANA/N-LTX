//! The voltik side of the conformance run (tools/conformance): converts every `<hash>.tex` of a folder
//! and writes `<hash>.png` at 100 dpi on white, as TeX's reference is drawn, plus one JSON line per
//! figure in `results.jsonl`: whether it converted, the warnings, the error, and the time.
//!
//! `cargo run --release -p voltik --example conformance -- <input dir> <output dir>`
//!
//! A panic or a figure that takes longer than 10 s is recorded instead of stopping the run.

use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

const DPI: f32 = 100.0;
const TIMEOUT: Duration = Duration::from_secs(10);

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

enum Outcome {
    Drawn { warnings: Vec<String>, svg_us: f64, png: Result<Vec<u8>, String> },
    Error(String),
    Panic(String),
}

/// Strokes thinner than a pixel drawn one pixel wide, as Ghostscript draws TeX's reference: the
/// comparison is about geometry, and a hairline would otherwise vanish in antialiasing.
fn hairlines_to_pixels(svg: &str) -> String {
    let pixel = 72.0 / f64::from(DPI);
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(i) = rest.find("stroke-width=\"") {
        let start = i + "stroke-width=\"".len();
        out.push_str(&rest[..start]);
        let end = rest[start..].find('"').map_or(rest.len(), |e| start + e);
        let value: f64 = rest[start..end].parse().unwrap_or(pixel);
        out.push_str(&format!("{}", value.max(pixel)));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// A figure that builds the engine of a thread.
const WARM_UP: &str = r"\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}";

fn run(code: String) -> Outcome {
    let caught = std::panic::catch_unwind(move || {
        // the thread's engine is built by a first conversion: what is timed is one in a running process
        let _ = voltik::convert(WARM_UP);
        let start = Instant::now();
        let r = voltik::convert(&code);
        let svg_us = start.elapsed().as_secs_f64() * 1e6;
        match r {
            Ok(out) => Outcome::Drawn { png: voltik::png(&hairlines_to_pixels(&out.svg), DPI, true).map_err(|e| e.to_string()), warnings: out.warnings, svg_us },
            Err(e) => Outcome::Error(e.to_string()),
        }
    });
    caught.unwrap_or_else(|p| Outcome::Panic(p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default()))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [input, output] = &args[..] else {
        eprintln!("usage: conformance <input dir> <output dir>");
        std::process::exit(2);
    };
    fs::create_dir_all(output).expect("output folder");
    std::panic::set_hook(Box::new(|_| {}));
    let mut files: Vec<_> = fs::read_dir(input).expect("input folder").filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "tex")).collect();
    files.sort();
    // the start of a process: the first figure builds the engine (its TeX definitions)
    let startup_us = thread::spawn(|| {
        let start = Instant::now();
        let _ = voltik::convert(WARM_UP);
        start.elapsed().as_secs_f64() * 1e6
    })
    .join()
    .unwrap_or(0.0);
    fs::write(format!("{output}/startup.json"), format!("{{\"startup_us\": {startup_us:.1}}}\n")).expect("write startup");
    let mut lines = String::new();
    for path in &files {
        let hash = path.file_stem().unwrap().to_string_lossy().to_string();
        let code = fs::read_to_string(path).unwrap_or_default();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(run(code));
        });
        let fields = match rx.recv_timeout(TIMEOUT) {
            Ok(Outcome::Drawn { warnings, svg_us, png }) => {
                let png_error = match png {
                    Ok(bytes) => {
                        fs::write(format!("{output}/{hash}.png"), bytes).expect("write png");
                        "null".to_string()
                    }
                    Err(e) => json_string(&e),
                };
                let w: Vec<String> = warnings.iter().map(|w| json_string(w)).collect();
                format!(r#""status": "drawn", "svg_us": {svg_us:.1}, "png_error": {png_error}, "warnings": [{}]"#, w.join(", "))
            }
            Ok(Outcome::Error(e)) => format!(r#""status": "error", "error": {}"#, json_string(&e)),
            Ok(Outcome::Panic(m)) => format!(r#""status": "panic", "error": {}"#, json_string(&m)),
            // the thread keeps running; the process exits at the end of the run anyway
            Err(_) => r#""status": "timeout""#.to_string(),
        };
        lines.push_str(&format!("{{\"hash\": {}, {fields}}}\n", json_string(&hash)));
    }
    fs::write(format!("{output}/results.jsonl"), lines).expect("write results");
    println!("{} figures converted into {output}", files.len());
}
