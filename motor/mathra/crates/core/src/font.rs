//! Math font embedded in the binary. Nothing is read from the user's system.
//! The embedded files are subsets of the originals in tools/fonts, written by tools/fonts/subset.sh.

use std::fmt::Write;
use std::sync::OnceLock;

use ttf_parser::math::{Constants, MathValue};
pub use ttf_parser::GlyphId;
use ttf_parser::{Face, OutlineBuilder};

static STIX_TWO: &[u8] = include_bytes!("../fonts/STIXTwoMath-Regular-subset.otf");
#[cfg(feature = "latin-modern")]
static LATIN_MODERN: &[u8] = include_bytes!("../fonts/latinmodern-math-subset.otf");
/// The name of the default font.
pub const NAME: &str = "STIX Two Math";

/// The math fonts mathra can typeset in: STIX Two Math (the default), and with the `latin-modern`
/// feature Latin Modern Math, the OpenType Computer Modern that TeX's own documents are set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MathFont {
    #[default]
    StixTwo,
    #[cfg(feature = "latin-modern")]
    LatinModern,
}

impl MathFont {
    pub fn name(self) -> &'static str {
        match self {
            MathFont::StixTwo => NAME,
            #[cfg(feature = "latin-modern")]
            MathFont::LatinModern => "Latin Modern Math",
        }
    }

    /// The space between words in text, in em: a quarter em in STIX Two Math, and in Latin Modern
    /// Math its space, as wide as Computer Modern's (a third of an em).
    pub fn word_space(self) -> f32 {
        match self {
            MathFont::StixTwo => 0.25,
            #[cfg(feature = "latin-modern")]
            MathFont::LatinModern => {
                let f = self.get();
                f.glyph(' ').map_or(0.25, |g| f.advance(g) / f.units_per_em())
            }
        }
    }

    /// How far a subscript without a superscript goes down, in em, where the font says otherwise
    /// than its MATH table: Latin Modern Math, which stands for Computer Modern, takes cmsy10's `sub1`
    /// (0.15 em) as pdfTeX does; its MATH table has only `sub2` (0.247 em), meant for a subscript
    /// under a superscript.
    pub fn lone_subscript_shift(self) -> Option<f32> {
        match self {
            MathFont::StixTwo => None,
            #[cfg(feature = "latin-modern")]
            MathFont::LatinModern => Some(0.15),
        }
    }

    /// How far a superscript goes up in display style, in em, where the font says otherwise than its
    /// MATH table: Latin Modern Math takes cmsy10's `sup1` (0.413 em), as pdfTeX does in display.
    pub fn display_superscript_shift(self) -> Option<f32> {
        match self {
            MathFont::StixTwo => None,
            #[cfg(feature = "latin-modern")]
            MathFont::LatinModern => Some(0.413),
        }
    }

    /// The room TeX leaves above an upper limit and below a lower one (`big_op_spacing5`), in em,
    /// for the font that stands for Computer Modern: cmex10's 0.1 em. The MATH table has no such
    /// constant.
    pub fn limit_padding(self) -> Option<f32> {
        match self {
            MathFont::StixTwo => None,
            #[cfg(feature = "latin-modern")]
            MathFont::LatinModern => Some(0.1),
        }
    }

    /// The font, parsed once.
    pub fn get(self) -> &'static Font {
        match self {
            MathFont::StixTwo => {
                static FONT: OnceLock<Font> = OnceLock::new();
                FONT.get_or_init(|| Font::parse(STIX_TWO))
            }
            #[cfg(feature = "latin-modern")]
            MathFont::LatinModern => {
                static FONT: OnceLock<Font> = OnceLock::new();
                FONT.get_or_init(|| Font::parse(LATIN_MODERN))
            }
        }
    }
}

pub struct Font {
    face: Face<'static>,
    /// The `ssty` alternates of a glyph: its forms for scripts and for scripts of scripts, drawn for
    /// those sizes as TeX draws its 7pt and 5pt designs (wider and bolder than the 10pt one scaled)
    script_forms: std::collections::HashMap<GlyphId, Vec<GlyphId>>,
}

/// The default font (STIX Two Math).
pub fn font() -> &'static Font {
    MathFont::StixTwo.get()
}

/// One part of an assembled glyph (tall parenthesis, long brace).
#[derive(Debug, Clone, Copy)]
pub struct Part {
    pub id: GlyphId,
    pub start: f32,
    pub end: f32,
    pub advance: f32,
    pub extender: bool,
}

impl Font {
    fn parse(data: &'static [u8]) -> Font {
        let face = Face::parse(data, 0).expect("valid embedded font");
        let script_forms = script_forms(&face);
        Font { face, script_forms }
    }

    /// The glyph to draw at a script level (1: script, 2: script of script): its `ssty` form for
    /// that level, the last one there is for deeper levels, or the glyph itself.
    pub fn script_form(&self, id: GlyphId, level: usize) -> GlyphId {
        if level == 0 {
            return id;
        }
        self.script_forms.get(&id).and_then(|forms| forms.get(level - 1).or(forms.last())).copied().unwrap_or(id)
    }

    pub fn units_per_em(&self) -> f32 {
        f32::from(self.face.units_per_em())
    }

    pub fn glyph(&self, c: char) -> Option<GlyphId> {
        self.face.glyph_index(c)
    }

    pub fn advance(&self, id: GlyphId) -> f32 {
        f32::from(self.face.glyph_hor_advance(id).unwrap_or(0))
    }

    /// (x_min, y_min, x_max, y_max) of the outline, in font units.
    pub fn limits(&self, id: GlyphId) -> (f32, f32, f32, f32) {
        match self.face.glyph_bounding_box(id) {
            Some(r) => (f32::from(r.x_min), f32::from(r.y_min), f32::from(r.x_max), f32::from(r.y_max)),
            None => (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// Glyph outline as the `d` of an SVG `<path>`, with y pointing up (as in the font).
    pub fn outline(&self, id: GlyphId) -> String {
        let mut path = PathWriter(String::new());
        self.face.outline_glyph(id, &mut path);
        path.0.trim_end().to_string()
    }

    fn constants(&self) -> Constants<'static> {
        self.face.tables().math.and_then(|m| m.constants).expect("the embedded font has a MATH table")
    }

    /// A constant from the MATH table, in font units.
    pub fn c(&self, f: impl Fn(&Constants<'static>) -> MathValue<'static>) -> f32 {
        f32::from(f(&self.constants()).value)
    }

    pub fn script_scale(&self) -> f32 {
        f32::from(self.constants().script_percent_scale_down()) / 100.0
    }

    pub fn script_script_scale(&self) -> f32 {
        f32::from(self.constants().script_script_percent_scale_down()) / 100.0
    }

    pub fn display_operator_min_height(&self) -> f32 {
        f32::from(self.constants().display_operator_min_height())
    }

    pub fn radical_degree_raise(&self) -> f32 {
        f32::from(self.constants().radical_degree_bottom_raise_percent()) / 100.0
    }

    pub fn italic_correction(&self, id: GlyphId) -> f32 {
        self.face.tables().math.and_then(|m| m.glyph_info).and_then(|g| g.italic_corrections).and_then(|c| c.get(id)).map_or(0.0, |v| f32::from(v.value))
    }

    pub fn accent_point(&self, id: GlyphId) -> Option<f32> {
        self.face.tables().math.and_then(|m| m.glyph_info).and_then(|g| g.top_accent_attachments).and_then(|c| c.get(id)).map(|v| f32::from(v.value))
    }

    pub fn min_overlap(&self) -> f32 {
        self.face.tables().math.and_then(|m| m.variants).map_or(0.0, |v| f32::from(v.min_connector_overlap))
    }

    /// Ready-made size variants, smallest to largest: (glyph, measure).
    pub fn variants(&self, id: GlyphId, vertical: bool) -> Vec<(GlyphId, f32)> {
        self.face
            .tables()
            .math
            .and_then(|m| m.variants)
            .and_then(|v| if vertical { v.vertical_constructions } else { v.horizontal_constructions }.get(id))
            .map(|c| c.variants.into_iter().map(|g| (g.variant_glyph, f32::from(g.advance_measurement))).collect())
            .unwrap_or_default()
    }

    /// Parts to assemble a glyph larger than every variant, bottom to top
    /// (vertical) or left to right (horizontal).
    pub fn assembly(&self, id: GlyphId, vertical: bool) -> Option<Vec<Part>> {
        let v = self.face.tables().math.and_then(|m| m.variants)?;
        let constructions = if vertical { v.vertical_constructions } else { v.horizontal_constructions };
        let assembly = constructions.get(id)?.assembly?;
        Some(
            assembly
                .parts
                .into_iter()
                .map(|p| Part {
                    id: p.glyph_id,
                    start: f32::from(p.start_connector_length),
                    end: f32::from(p.end_connector_length),
                    advance: f32::from(p.full_advance),
                    extender: p.part_flags.extender(),
                })
                .collect(),
        )
    }
}

/// The alternates of the GSUB `ssty` feature, by glyph.
fn script_forms(face: &Face<'static>) -> std::collections::HashMap<GlyphId, Vec<GlyphId>> {
    use ttf_parser::gsub::SubstitutionSubtable;
    let mut forms = std::collections::HashMap::new();
    let Some(gsub) = face.tables().gsub else { return forms };
    for feature in gsub.features.into_iter().filter(|f| f.tag == ttf_parser::Tag::from_bytes(b"ssty")) {
        for index in feature.lookup_indices {
            let Some(lookup) = gsub.lookups.get(index) else { continue };
            for subtable in lookup.subtables.into_iter::<SubstitutionSubtable>() {
                let SubstitutionSubtable::Alternate(alternate) = subtable else { continue };
                for (i, set) in alternate.alternate_sets.into_iter().enumerate() {
                    if let Some(glyph) = coverage_glyph(&alternate.coverage, i as u16) {
                        forms.entry(glyph).or_insert_with(|| set.alternates.into_iter().collect());
                    }
                }
            }
        }
    }
    forms
}

/// The glyph a coverage table lists at `index`.
fn coverage_glyph(coverage: &ttf_parser::opentype_layout::Coverage, index: u16) -> Option<GlyphId> {
    use ttf_parser::opentype_layout::Coverage;
    match coverage {
        Coverage::Format1 { glyphs } => glyphs.get(index),
        Coverage::Format2 { records } => records.into_iter().find_map(|r| {
            let first = r.value;
            (index >= first && index - first <= r.end.0 - r.start.0).then(|| GlyphId(r.start.0 + index - first))
        }),
    }
}

struct PathWriter(String);

fn num(v: f32) -> String {
    let rounded = (v * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded}")
    }
}

impl OutlineBuilder for PathWriter {
    fn move_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "M{} {} ", num(x), num(y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.0, "L{} {} ", num(x), num(y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let _ = write!(self.0, "Q{} {} {} {} ", num(x1), num(y1), num(x), num(y));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let _ = write!(self.0, "C{} {} {} {} {} {} ", num(x1), num(y1), num(x2), num(y2), num(x), num(y));
    }
    fn close(&mut self) {
        self.0.push_str("Z ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_without_outline_has_zero_bounds() {
        let f = font();
        assert_eq!(f.limits(f.glyph(' ').unwrap()), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn quadratic_curve_becomes_q() {
        // STIX Two Math is CFF (cubics only), but the outline writer also accepts TrueType fonts
        let mut c = PathWriter(String::new());
        c.quad_to(1.0, 2.25, 3.0, 4.0);
        assert_eq!(c.0, "Q1 2.3 3 4 ");
    }
}
