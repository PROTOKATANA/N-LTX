//! The metrics of TeX's fonts that `em` and `ex` read and text is measured with.

/// How much wider than its 10pt design TeX's font of `pt` points sets text, in the family the text
/// is in: LaTeX picks the Computer Modern of the design size at or below it (`cmr7` for
/// `\scriptsize`, `cmr17` for `\Huge`), whose letters are proportionally wider at small sizes and
/// narrower at large ones, while the label font has only the 10pt design. From the fonts' metrics.
pub fn design_width(pt: f64, family: char) -> f64 {
    let table: &[(f64, f64)] = match family {
        // cmtt8, cmtt9, cmtt10 and cmtt12
        't' => &[(0.0, 1.0119), (9.0, 1.0), (10.0, 1.0), (12.0, 0.9802)],
        // cmmi5 … cmmi12
        'm' => &[(0.0, 1.4283), (6.0, 1.2526), (7.0, 1.1559), (8.0, 1.0584), (9.0, 1.026), (10.0, 1.0), (12.0, 0.9784)],
        // cmss8 (from 5pt) … cmss17
        's' => &[(0.0, 1.0625), (9.0, 1.0278), (10.0, 1.0), (12.0, 0.9792), (17.0, 0.9404)],
        // cmbx5 … cmbx12
        'b' => &[(0.0, 1.3083), (6.0, 1.1996), (7.0, 1.1221), (8.0, 1.064), (9.0, 1.0285), (10.0, 1.0), (12.0, 0.9774)],
        // cmr5 … cmr17
        _ => &[(0.0, 1.3611), (6.0, 1.2222), (7.0, 1.1389), (8.0, 1.0625), (9.0, 1.0278), (10.0, 1.0), (12.0, 0.9791), (17.0, 0.9172)],
    };
    table.iter().rev().find(|(from, _)| pt + 0.01 >= *from).map_or(1.0, |(_, w)| *w)
}

/// The quad (`1em`) of TeX's roman font of `pt` points, or its sans serif one, over `pt`.
pub fn quad(pt: f64, sans: bool) -> f64 {
    design_width(pt, if sans { 's' } else { 'r' })
}

/// The x-height (`1ex`) of TeX's roman font, or its sans serif one, over `pt`.
pub fn x_height(sans: bool) -> f64 {
    if sans {
        0.4444
    } else {
        0.4306
    }
}
