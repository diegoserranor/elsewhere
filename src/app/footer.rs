use gpui::{Context, Window, div, prelude::*, rgb};

use super::Elsewhere;
use crate::theme;

impl Elsewhere {
    fn unpin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pinned = None;
        self.close_editor(window, cx);
    }

    fn toggle_westward(&mut self, cx: &mut Context<Self>) {
        self.westward = !self.westward;
        cx.notify();
    }

    /// The strip along the bottom: a slot on the left for whatever the moment
    /// calls for, view preferences on the right. It is always there, at a
    /// fixed height, so the list above never shifts as controls come and go.
    pub(super) fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .h_5()
            .text_xs()
            .child(div().children(self.pinned.is_some().then(|| {
                div()
                    .id("unpin")
                    .px_1()
                    .rounded_md()
                    .cursor_pointer()
                    .text_color(rgb(theme::YELLOW))
                    .hover(|style| style.bg(rgb(theme::SURFACE0)))
                    .on_click(cx.listener(|this, _event, window, cx| this.unpin(window, cx)))
                    .child("back to now")
            })))
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
