//! TeX groups and pgf scopes: what a group saves and what a scope closes.

use super::PgfScope;
use super::*;
use crate::canvas::Item;
use crate::scene::Point;
use crate::tex::Token;
use crate::Error;

impl Engine {
    // ---------------------------------------------------------------- groups and scopes

    pub fn begin_group(&mut self) {
        self.tex.begin_group();
        self.saved.push(self.st.clone());
    }

    pub fn end_group(&mut self) {
        if self.tex.end_group() {
            if let Some(s) = self.saved.pop() {
                self.st = s;
            }
            self.canvas.overlay = self.st.overlay;
            self.close_local_boxes();
        } else {
            self.warn("extra '}'");
        }
    }

    /// `local bounding box=name`: a rectangle node `name` over what is drawn from here to the end
    /// of the group.
    pub fn open_local_box(&mut self, name: String) {
        self.scoped.local_boxes.push((name, self.saved.len()));
        self.canvas.local.push((None, None));
    }

    /// The local bounding boxes of the groups that ended, as nodes.
    pub(super) fn close_local_boxes(&mut self) {
        while self.scoped.local_boxes.last().is_some_and(|b| b.1 > self.saved.len()) {
            let (name, _) = self.scoped.local_boxes.pop().expect("checked");
            if let Some((Some(lo), Some(hi))) = self.canvas.local.pop() {
                self.nodes.insert(name, super::libraries::box_node(lo, hi));
            }
        }
    }

    pub fn begin_scope(&mut self) {
        self.scopes.push(PgfScope::default());
    }

    pub fn end_scope(&mut self) {
        if let Some(s) = self.scopes.pop() {
            for _ in 0..s.ends {
                self.canvas.push(Item::End);
            }
        }
    }

    /// Runs `code` unseen, as [`crate::extension::Context::measure`]: in a group and a scope of its
    /// own, on a canvas apart, `read` called at its end, and then the nodes, the named paths and the
    /// layers as they were before. The box is that of what `code` drew, on the canvas.
    pub fn run_unseen<T>(&mut self, code: Vec<Token>, read: impl FnOnce(&mut Engine) -> T) -> Result<(T, Option<(Point, Point)>), Error> {
        let saved_canvas = std::mem::take(&mut self.canvas);
        let saved_frozen = std::mem::replace(&mut self.canvas_frozen, false);
        let saved_layers = std::mem::take(&mut self.layering.layers);
        let saved_behind = std::mem::take(&mut self.node_run.behind_path);
        let nodes = self.nodes.clone();
        let named_paths = self.pgf.named_paths.clone();
        let last_node = (self.last_node.clone(), self.last_node_name.clone());
        self.begin_group();
        self.begin_scope();
        let r = self.run_tokens(code);
        let value = r.is_ok().then(|| read(self));
        self.end_scope();
        self.end_group();
        let drawn = std::mem::replace(&mut self.canvas, saved_canvas);
        self.canvas_frozen = saved_frozen;
        self.layering.layers = saved_layers;
        self.node_run.behind_path = saved_behind;
        self.nodes = nodes;
        self.pgf.named_paths = named_paths;
        (self.last_node, self.last_node_name) = last_node;
        r?;
        Ok((value.expect("read after the code ran"), drawn.min.zip(drawn.max)))
    }

    /// Opens a canvas item that the current pgf scope closes (a clip, a transparency group).
    pub fn open_in_scope(&mut self, item: Item) {
        let fading = matches!(item, Item::Fade { .. } | Item::FadePicture { .. });
        // a fading replaces the scope's fading, as pgf's graphic state holds one
        if fading && self.scopes.last().is_some_and(|s| s.fading_last && s.ends > 0) {
            self.canvas.push(Item::End);
            if let Some(s) = self.scopes.last_mut() {
                s.ends -= 1;
            }
        }
        self.canvas.push(item);
        match self.scopes.last_mut() {
            Some(s) => {
                s.ends += 1;
                s.fading_last = fading;
            }
            None => self.canvas.push(Item::End),
        }
    }
}
