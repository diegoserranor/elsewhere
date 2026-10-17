use gpui::{
    Context, MouseButton, MouseDownEvent, Pixels, Window, div, prelude::*, px, rgb, svg,
    transparent_black,
};

use super::Elsewhere;
use crate::theme;

/// A window control, as a footer-sized icon button.
struct Control {
    id: &'static str,
    icon: &'static str,
    size: Pixels,
    /// The icon's color under the pointer.
    hover: u32,
    /// A plain fn, so the element can outlive the borrow that built it.
    click: fn(&mut Window, &mut Context<Elsewhere>),
}

const MINIMIZE: Control = Control {
    id: "minimize",
    icon: "icons/minus.svg",
    size: px(14.),
    hover: theme::TEXT,
    click: |window, _| window.minimize_window(),
};

const MAXIMIZE: Control = Control {
    id: "maximize",
    icon: "icons/maximize-2.svg",
    size: px(12.),
    hover: theme::TEXT,
    click: |window, _| window.zoom_window(),
};

const RESTORE: Control = Control {
    icon: "icons/minimize-2.svg",
    ..MAXIMIZE
};

const CLOSE: Control = Control {
    id: "close",
    icon: "icons/x.svg",
    size: px(14.),
    hover: theme::RED,
    // The one window is the whole app.
    click: |_, cx| cx.quit(),
};

impl Elsewhere {
    fn render_control(&self, control: &Control, cx: &mut Context<Self>) -> impl IntoElement {
        let Control {
            id,
            icon,
            size,
            hover,
            click,
        } = *control;
        div()
            .id(id)
            .group(id)
            .w_7()
            .h_5()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            // The same invisible border as the footer's buttons, so the icons
            // sit in the same box.
            .border_1()
            .border_color(transparent_black())
            .cursor_pointer()
            .hover(|style| style.bg(rgb(theme::SURFACE0)))
            .on_click(cx.listener(move |_, _event, window, cx| click(window, cx)))
            .child(
                svg()
                    .path(icon)
                    .size(size)
                    // An svg paints only in a color set on itself; the
                    // parent's does not reach it.
                    .text_color(rgb(theme::OVERLAY0))
                    .group_hover(id, |style| style.text_color(rgb(hover))),
            )
    }

    /// The strip along the top that stands in for the title bar the window
    /// goes without: the app's name and empty space that move the window, a
    /// double click on them maximizing, and the window's controls on the right.
    pub(super) fn render_titlebar(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let maximize = if window.is_maximized() {
            &RESTORE
        } else {
            &MAXIMIZE
        };
        div()
            .flex()
            .flex_row()
            .items_center()
            .h_5()
            // The name rides inside the empty space, so it moves the window too.
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(rgb(theme::OVERLAY0))
                    .child("elsewhere")
                    .on_mouse_down(MouseButton::Left, |event: &MouseDownEvent, window, _| {
                        if event.click_count == 2 {
                            window.zoom_window();
                        } else {
                            window.start_window_move();
                        }
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .child(self.render_control(&MINIMIZE, cx))
                    .child(self.render_control(maximize, cx))
                    .child(self.render_control(&CLOSE, cx)),
            )
    }
}
