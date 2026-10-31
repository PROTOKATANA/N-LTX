//! PNG images as `\includegraphics` finds them, and the projection `\addplot graphics` solves.

use std::rc::Rc;

use crate::PT;

/// Largest image read.
pub const MAX_IMAGE: u64 = 16 << 20;

/// One bp, in cm.
pub const BP: f64 = 2.54 / 72.0;

/// A point of `points`: data coordinates (z NaN in 2d), and image coordinates (pt) if mapped.
pub type MapPoint = ([f64; 3], Option<(f64, f64)>);

#[derive(Debug, Clone, Default)]
pub struct Graphics {
    /// The PNG file, if it was found and read.
    pub png: Option<Rc<Vec<u8>>>,
    /// Its natural size (cm), as pdfTeX takes it from its pixels and resolution.
    pub natural: (f64, f64),
    /// `includegraphics={trim=l b r t}` (cm) and `clip`.
    pub trim: [f64; 4],
    pub clip: bool,
    /// `points`.
    pub points: Vec<MapPoint>,
}

/// The size in pixels and the resolution (dots per inch, 0 if none) of a PNG file.
pub fn png_info(data: &[u8]) -> Option<(u32, u32, f64, f64)> {
    if data.len() < 24 || &data[..8] != b"\x89PNG\r\n\x1a\n" || &data[12..16] != b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    let (w, h) = (be(16), be(20));
    let mut res = (0.0, 0.0);
    let mut i = 8;
    while i + 8 <= data.len() {
        let len = be(i) as usize;
        let kind = &data[i + 4..i + 8];
        if kind == b"IDAT" {
            break;
        }
        // pixels per meter, unit 1 = meter: pdfTeX rounds to dots per inch
        if kind == b"pHYs" && len == 9 && i + 17 <= data.len() && data[i + 16] == 1 {
            res = ((f64::from(be(i + 8)) * 0.0254).round(), (f64::from(be(i + 12)) * 0.0254).round());
        }
        i += 12 + len;
    }
    Some((w, h, res.0, res.1))
}

/// The image's natural size in cm: pixels at their resolution, 72 dpi without one (pdfTeX's
/// `\pdfimageresolution` default).
pub fn natural_size(w: u32, h: u32, xres: f64, yres: f64) -> (f64, f64) {
    let (xr, yr) = if xres > 0.0 && yres > 0.0 { (xres, yres) } else { (72.0, 72.0) };
    (f64::from(w) * 72.27 / xr * PT, f64::from(h) * 72.27 / yr * PT)
}

/// The file `\includegraphics` would load: the name as given, or with the first of pdfTeX's
/// extensions that exists (`\Gin@extensions`).
pub fn find_image(name: &str) -> Result<String, String> {
    let n = name.trim();
    let base = n.rsplit('/').next().unwrap_or(n);
    if base.contains('.') {
        return Ok(n.to_string());
    }
    for ext in [".pdf", ".PDF", ".ai", ".AI", ".png", ".PNG", ".jpg", ".JPG", ".jpeg", ".JPEG", ".jbig2", ".JBIG2", ".jb2", ".JB2"] {
        let full = format!("{n}{ext}");
        if crate::files::find_file(&full, &[]).is_ok() {
            return Ok(full);
        }
    }
    Err(format!("file '{n}' not found"))
}

/// A PNG file's bytes and its natural size (cm).
pub type Png = (Rc<Vec<u8>>, (f64, f64));

/// A PNG image as `\includegraphics` would find it, and its natural size.
pub fn load_png(name: &str) -> Result<Png, String> {
    let file = find_image(name)?;
    let data = crate::files::read_bytes(&file, &[], MAX_IMAGE)?;
    match png_info(&data) {
        Some((w, h, xr, yr)) if w > 0 && h > 0 => Ok((Rc::new(data), natural_size(w, h, xr, yr))),
        _ => Err(format!("'{file}' is not a PNG image; only PNG images are drawn")),
    }
}
/// The text inside the parentheses `s` starts with, and what follows them.
pub fn paren_group(s: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[1..i], &s[i + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// Splits at the commas outside parentheses and braces.
pub fn split_top(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0i32, 0);
    for (i, c) in s.char_indices() {
        match c {
            '(' | '{' => depth += 1,
            ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}
/// Gaussian elimination with partial pivoting of an 8 × 8 system (the last column the right side).
pub fn solve(mut m: [[f64; 9]; 8]) -> Option<[f64; 8]> {
    for c in 0..8 {
        let p = (c..8).max_by(|&a, &b| m[a][c].abs().total_cmp(&m[b][c].abs()))?;
        if m[p][c].abs() < 1e-12 {
            return None;
        }
        m.swap(c, p);
        for r in 0..8 {
            if r != c {
                let k = m[r][c] / m[c][c];
                let pivot = m[c];
                for (x, p) in m[r].iter_mut().zip(pivot).skip(c) {
                    *x -= k * p;
                }
            }
        }
    }
    let mut x = [0.0; 8];
    for (i, v) in x.iter_mut().enumerate() {
        *v = m[i][8] / m[i][i];
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_a_projection() {
        // T(x,y,z) = (1,2) + x (3,1) + y (-2,1) + z (0,4)
        let t = |p: [f64; 3]| (1.0 + 3.0 * p[0] - 2.0 * p[1], 2.0 + p[0] + p[1] + 4.0 * p[2]);
        let pts = [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.5, 0.5, 1.8], [0.0, 0.0, 0.0]];
        let mut m = [[0.0; 9]; 8];
        for (k, p) in pts.iter().enumerate() {
            let (ix, iy) = t(*p);
            m[2 * k] = [p[0], 0.0, p[1], 0.0, p[2], 0.0, 1.0, 0.0, ix];
            m[2 * k + 1] = [0.0, p[0], 0.0, p[1], 0.0, p[2], 0.0, 1.0, iy];
        }
        let s = solve(m).expect("solvable");
        let want = [3.0, 1.0, -2.0, 1.0, 0.0, 4.0, 1.0, 2.0];
        for (a, b) in s.iter().zip(want) {
            assert!((a - b).abs() < 1e-9, "{s:?}");
        }
    }

    #[test]
    fn reads_png_resolution() {
        let mut d = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\x03\xa3\0\0\x01\xb1\x08\x06\0\0\0\0\0\0\0".to_vec();
        d.extend(b"\0\0\0\x09pHYs\0\0\x17\x12\0\0\x17\x12\x01\0\0\0\0");
        let (w, h, xr, yr) = png_info(&d).expect("a png");
        assert_eq!((w, h, xr, yr), (931, 433, 150.0, 150.0));
        // pdfTeX: 448.55469pt × 208.61888pt
        let (nw, nh) = natural_size(w, h, xr, yr);
        assert!((nw / PT - 448.5547).abs() < 1e-2 && (nh / PT - 208.6189).abs() < 1e-2);
    }
}
