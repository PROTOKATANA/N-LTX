/// The limits `mesh input=image` asks for, as pgfplots' heuristic finds them while it reads the
/// points: half a step before the first point of each scanline and after its last (along the
/// scanlines, axis `a`), and half a step beyond the first point of each scanline but the first, on
/// both sides (across them). `pts` are the centers, scanline by scanline.
pub fn image_limits(pts: &[[f64; 3]], rows: usize, cols: usize, a: usize) -> Vec<[f64; 3]> {
    let b = 1 - a;
    let mut out = Vec::new();
    let point = |va: f64, vb: f64| {
        let mut p = [0.0; 3];
        p[a] = va;
        p[b] = vb;
        p
    };
    let mut last_b: Option<f64> = None;
    let mut b_next = false;
    for s in 0..rows {
        let (mut last_a, mut lastlast_a): (Option<f64>, Option<f64>) = (None, None);
        let mut a_next = true;
        let mut cur_b = f64::NAN;
        for k in 0..cols {
            let Some(c) = pts.get(s * cols + k) else { break };
            let (ca, cb) = (c[a], c[b]);
            cur_b = cb;
            if let (true, Some(la)) = (a_next, last_a) {
                out.push(point(la - 0.5 * (ca - la), cb));
                a_next = false;
            }
            if let (true, Some(lb)) = (b_next, last_b) {
                out.push(point(ca, lb - 0.5 * (cb - lb)));
                out.push(point(ca, cb + 0.5 * (cb - lb)));
                b_next = false;
            }
            lastlast_a = last_a;
            last_a = Some(ca);
            last_b = Some(cb);
        }
        if let (Some(la), Some(lla)) = (last_a, lastlast_a) {
            out.push(point(la + 0.5 * (la - lla), cur_b));
        }
        b_next = true;
    }
    out
}

/// `mesh input=image` with `shader=interp`: the point metas of each cell's corners, as
/// `pgfplotsmeshplotimage.code.tex` sweeps the scanlines. A cell's first corner has its own meta;
/// an inner corner the meta of the center after it if bounded, else the mean of the four centers
/// around; a corner of the first row or column the meta of its center after it, else an
/// extrapolation; those of the last row and column extrapolations (`2 × mean of two centers −
/// the corner before`), the very last one the last center's. An unbounded meta counts as the
/// lowest (`\pgfplots@metamin`).
pub fn image_interp_metas(metas: &[f64], rows: usize, cols: usize, min: f64) -> Vec<[f64; 4]> {
    if rows < 2 || cols < 2 || metas.len() < rows * cols {
        return metas.iter().map(|&m| [m; 4]).collect();
    }
    let raw = |r: usize, c: usize| metas[r * cols + c];
    let m = |r: usize, c: usize| Some(raw(r, c)).filter(|v| v.is_finite()).unwrap_or(min);
    let or = |r: usize, c: usize, v: f64| Some(raw(r, c)).filter(|v| v.is_finite()).unwrap_or(v);
    // k[at(a, b)]: the corner between center rows a-1, a and columns b-1, b
    let mut k = vec![0.0; (rows + 1) * (cols + 1)];
    let at = |a: usize, b: usize| a * (cols + 1) + b;
    for a in 1..rows {
        for b in 1..cols {
            k[at(a, b)] = or(a, b, 0.25 * (m(a - 1, b - 1) + m(a - 1, b) + m(a, b) + m(a, b - 1)));
        }
    }
    for b in 1..cols {
        k[at(0, b)] = or(0, b, m(0, b) + m(0, b - 1) - k[at(1, b)]);
    }
    for a in 1..rows {
        k[at(a, 0)] = or(a, 0, m(a, 0) + m(a - 1, 0) - k[at(a, 1)]);
    }
    for a in 1..rows {
        k[at(a, cols)] = m(a - 1, cols - 1) + m(a, cols - 1) - k[at(a, cols - 1)];
    }
    k[at(0, cols)] = k[at(1, cols)] + k[at(0, cols - 1)] - k[at(1, cols - 1)];
    for b in 1..cols {
        k[at(rows, b)] = m(rows - 1, b - 1) + m(rows - 1, b) - k[at(rows - 1, b)];
    }
    k[at(rows, 0)] = k[at(rows, 1)] + k[at(rows - 1, 0)] - k[at(rows - 1, 1)];
    k[at(rows, cols)] = or(rows - 1, cols - 1, k[at(rows, cols - 1)] - k[at(rows - 1, cols - 1)] + k[at(rows - 1, cols)]);
    let mut out = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        for c in 0..cols {
            out.push([raw(r, c), k[at(r, c + 1)], k[at(r + 1, c + 1)], k[at(r + 1, c)]]);
        }
    }
    out
}

/// `mesh input=image`: the corners of the cells around a lattice of centers, (rows+1) × (cols+1),
/// halfway between neighbours and as far out at the borders.
pub fn image_corners(pts: &[[f64; 3]], rows: usize, cols: usize) -> Vec<[f64; 3]> {
    let c = |r: isize, k: isize| -> [f64; 3] {
        // a center, extrapolated linearly outside the lattice
        let (rr, kk) = (r.clamp(0, rows as isize - 1), k.clamp(0, cols as isize - 1));
        let get = |r: isize, k: isize| pts.get(r as usize * cols + k as usize).copied().unwrap_or([f64::NAN; 3]);
        let base = get(rr, kk);
        let mut v = base;
        if r != rr && rows > 1 {
            let other = get(if r < 0 { 1 } else { rows as isize - 2 }, kk);
            for i in 0..3 {
                v[i] += base[i] - other[i];
            }
        }
        if k != kk && cols > 1 {
            let other = get(rr, if k < 0 { 1 } else { cols as isize - 2 });
            for i in 0..3 {
                v[i] += base[i] - other[i];
            }
        }
        v
    };
    let mut out = Vec::with_capacity((rows + 1) * (cols + 1));
    for r in 0..=rows as isize {
        for k in 0..=cols as isize {
            let q = [c(r - 1, k - 1), c(r - 1, k), c(r, k - 1), c(r, k)];
            out.push([0, 1, 2].map(|i| q.iter().map(|p| p[i]).sum::<f64>() / 4.0));
        }
    }
    out
}
