use gpui::{Context, Div, Window, div, prelude::*, rgb, transparent_black};

use super::Elsewhere;
use crate::saved;
use crate::theme;

/// The row that follows the cursor during a drag.
pub(super) struct DragRow {
    pub(super) geonameid: u32,
    pub(super) label: String,
}

impl Render for DragRow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgb(theme::MANTLE))
            .text_color(rgb(theme::SUBTEXT0))
            .opacity(0.9)
            .shadow_md()
            .child(self.label.clone())
    }
}

/// Dresses an element as a landing place for a dragged row: a transparent top
/// border that lights up as the insertion line. The caller attaches the
/// `on_drop` that says where the row lands.
pub(super) fn drop_target(element: Div) -> Div {
    element
        .border_t_2()
        .border_color(transparent_black())
        .drag_over::<DragRow>(|style, _, _, _| style.border_color(rgb(theme::BLUE)))
}

impl Elsewhere {
    /// Lands a dragged row before `before`, or at the end without one.
    pub(super) fn drop(&mut self, dragged: u32, before: Option<u32>, cx: &mut Context<Self>) {
        self.drag = None;
        if saved::reorder(&mut self.saved, dragged, before) {
            saved::save(&self.saved);
        }
        cx.notify();
    }

    /// The space below the list catches a drop meant for the end.
    pub(super) fn render_drop_tail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        drop_target(div().flex_1().min_h_4()).on_drop(
            cx.listener(|this, row: &DragRow, _window, cx| this.drop(row.geonameid, None, cx)),
        )
    }
}
