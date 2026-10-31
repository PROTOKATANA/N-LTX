//! Benchmark over the hand-made figures in `corpus/`:
//! `cargo run --release -p tikra --example bench`.
//!
//! Prints how many figures convert without warnings, with warnings or with an error, and the time
//! per figure for SVG and for PNG at 300 dpi (mean and 95th percentile of the per-figure medians).

use std::time::Instant;

const RUNS: usize = 15;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn summary(mut v: Vec<f64>) -> (f64, f64) {
    v.sort_by(f64::total_cmp);
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let p95 = v[((v.len() as f64 * 0.95).ceil() as usize).saturating_sub(1)];
    (mean, p95)
}

fn main() {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus"));
    let mut files = Vec::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(d) = folders.pop() {
        for p in std::fs::read_dir(&d).expect("corpus/ exists").filter_map(Result::ok).map(|e| e.path()) {
            if p.is_dir() {
                folders.push(p);
            } else if p.extension().is_some_and(|x| x == "tex") {
                files.push(p);
            }
        }
    }
    files.sort();
    let (mut clean, mut warned, mut failed) = (0, 0, 0);
    let (mut svg_ms, mut png_ms) = (Vec::new(), Vec::new());
    for f in &files {
        let tex = std::fs::read_to_string(f).unwrap();
        let out = match tikra::convert(&tex) {
            Ok(o) => o,
            Err(e) => {
                failed += 1;
                eprintln!("error  {}: {e}", f.display());
                continue;
            }
        };
        if out.warnings.is_empty() {
            clean += 1;
        } else {
            warned += 1;
            eprintln!("warning  {}: {}", f.display(), out.warnings.join("; "));
        }
        let time = |job: &dyn Fn()| {
            median(
                (0..RUNS)
                    .map(|_| {
                        let t = Instant::now();
                        job();
                        t.elapsed().as_secs_f64() * 1000.0
                    })
                    .collect(),
            )
        };
        svg_ms.push(time(&|| {
            tikra::convert(&tex).unwrap();
        }));
        png_ms.push(time(&|| {
            tikra::png(&tikra::convert(&tex).unwrap().svg, 300.0, true).unwrap();
        }));
    }
    let (svg_mean, svg_p95) = summary(svg_ms);
    let (png_mean, png_p95) = summary(png_ms);
    println!("| Figures | Without warnings | With warnings | Errors |");
    println!("|---:|---:|---:|---:|");
    println!("| {} | {clean} | {warned} | {failed} |", files.len());
    println!();
    println!("| Per figure | Mean | p95 |");
    println!("|---|---:|---:|");
    println!("| SVG | {svg_mean:.2} ms | {svg_p95:.2} ms |");
    println!("| SVG + PNG at 300 dpi | {png_mean:.2} ms | {png_p95:.2} ms |");
}
