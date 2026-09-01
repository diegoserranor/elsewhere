use gpui::{Context, Window, div, prelude::*, rgb, rgba};
use jiff::Zoned;

use super::{Elsewhere, row};
use crate::clock;
use crate::theme;

impl Elsewhere {
    pub(super) fn unpin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pinned = None;
        self.close_editor(window, cx);
    }

    fn toggle_westward(&mut self, cx: &mut Context<Self>) {
        self.westward = !self.westward;
        cx.notify();
    }

    /// The strip under the search that says a what-if time is in force: which
    /// row it was typed into, what it reads there, and the way out. Only
    /// there while pinned; entering a mode on purpose earns the shift.
    pub(super) fn render_banner(
        &self,
        now: &Zoned,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let pin = self.pinned.as_ref()?;
        let city = self.index.city(pin.anchor)?;
        let time = self
            .zones
            .get(&city.timezone)
            .map(|zone| clock::reading(now, zone).time)
            .unwrap_or_else(|| clock::UNKNOWN.to_string());
        Some(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
                .mr(row::GUTTER)
                .h_6()
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(rgba(theme::YELLOW_EDGE))
                .bg(rgba(theme::YELLOW_TINT))
                .text_xs()
                .text_color(rgb(theme::YELLOW))
                .child(format!("viewing {time} in {}", city.name))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1p5()
                        .child(
                            div()
                                .id("unpin")
                                .px_1()
                                .rounded_md()
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(theme::SURFACE0)))
                                .on_click(
                                    cx.listener(|this, _event, window, cx| this.unpin(window, cx)),
                                )
                                .child("back to now"),
                        )
                        .child(div().text_color(rgb(theme::OVERLAY0)).child("esc")),
                ),
        )
    }

    /// The strip along the bottom: a free slot on the left, view preferences
    /// on the right. It is always there, at a
    /// fixed height, so the list above never shifts as controls come and go.
    pub(super) fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .h_5()
            .text_xs()
            // The left slot is free for now.
            .child(div())
            .child(div().children((self.saved.len() > 1).then(|| {
                div()
                    .id("westward")
                    .px_1()
                    .rounded_md()
                    .cursor_pointer()
                    .text_color(if self.westward {
                        rgb(theme::BLUE)
                    } else {
                        rgb(theme::OVERLAY0)
                    })
                    .hover(|style| style.bg(rgb(theme::SURFACE0)))
                    .on_click(cx.listener(|this, _event, _window, cx| this.toggle_westward(cx)))
                    .child("west → east")
            })))
    }
}
