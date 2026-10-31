use std::process::ExitCode;
use std::{env, fs, io};

const HELP: &str = "\
voltik — CircuiTikZ circuits to SVG and PNG, no TeX install

USAGE
  voltik <file.tex> [-o out.svg|out.png] [options]
  cat circuit.tex | voltik -o circuit.png

OPTIONS
  -o, --output <file>     .svg or .png (by its name); without -o, prints the SVG
      --dpi <n>           PNG resolution (default 300)
      --transparent       PNG without the white background
      --warnings          lists what was ignored or approximated
      --describe          prints the figure in words: its alt text and its description
      --animate           the SVG draws itself in the browser, in the order TikZ drew it
      --steps             each element numbered by its step (data-step), with no animation
  -l, --locale <code>     the language of the figure in words: en (default), pt-BR, es, fr, de,
                          it, ru, zh, ja, ko, hi, ar, id, tr, nl, pl (the drawing is the same)
  -V, --version
  -h, --help
";

fn main() -> ExitCode {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut dpi = 300.0f32;
    let mut background = true;
    let mut show_warnings = false;
    let mut describe = false;
    let mut options = voltik::Options::default();
    let mut it = env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("voltik {}", voltik::VERSION);
                return ExitCode::SUCCESS;
            }
            "-o" | "--output" => output = it.next(),
            "--dpi" => dpi = it.next().and_then(|v| v.parse().ok()).unwrap_or(300.0),
            "--transparent" => background = false,
            "--warnings" => show_warnings = true,
            "--describe" => describe = true,
            "--animate" => options = options.animated(),
            "--steps" => options = options.steps(),
            "-l" | "--locale" => match it.next().unwrap_or_default().parse() {
                Ok(l) => options.locale = l,
                Err(e) => {
                    eprintln!("voltik: {e}");
                    return ExitCode::from(2);
                }
            },
            _ => input = Some(a),
        }
    }
    let code = match &input {
        Some(c) => match fs::read_to_string(c) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("voltik: could not read {c}: {e}");
                return ExitCode::from(1);
            }
        },
        None => match io::read_to_string(io::stdin()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("voltik: could not read standard input: {e}");
                return ExitCode::from(1);
            }
        },
    };
    let r = match voltik::convert_with(&code, &options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("voltik: {e}");
            return ExitCode::from(1);
        }
    };
    for w in &r.warnings {
        if show_warnings {
            println!("warning: {w}");
        } else {
            eprintln!("voltik: warning: {w}");
        }
    }
    if describe {
        println!("alt: {}", r.alt);
        println!("description: {}", r.description);
    }
    match output {
        Some(s) if s.ends_with(".png") => match voltik::png(&r.svg, dpi, background) {
            Ok(b) => {
                if let Err(e) = fs::write(&s, b) {
                    eprintln!("voltik: could not write {s}: {e}");
                    return ExitCode::from(1);
                }
            }
            Err(e) => {
                eprintln!("voltik: {e}");
                return ExitCode::from(1);
            }
        },
        Some(s) => {
            if let Err(e) = fs::write(&s, &r.svg) {
                eprintln!("voltik: could not write {s}: {e}");
                return ExitCode::from(1);
            }
        }
        None if !show_warnings && !describe => println!("{}", r.svg),
        None => {}
    }
    ExitCode::SUCCESS
}
