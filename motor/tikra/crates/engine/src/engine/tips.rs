//! What the arrow tips ask of the engine: their values, read as TikZ reads them.

use tikra_arrows::Eval;

use super::Engine;
use crate::color::Color;

impl Eval for Engine {
    fn dim(&mut self, text: &str) -> Option<f64> {
        self.length(text).ok()
    }
    fn num(&mut self, text: &str) -> Option<f64> {
        self.number(text).ok()
    }
    fn paint(&mut self, text: &str) -> Option<Color> {
        // pgf's names for the path's current colors (`fill=pgffillcolor`)
        match text.trim() {
            "pgffillcolor" => Some(self.st.fill_paint_color()),
            "pgfstrokecolor" => Some(self.st.stroke_color()),
            t => self.color(t),
        }
    }
}
