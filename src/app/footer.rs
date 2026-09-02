use gpui::{Context, Pixels, Window, div, prelude::*, px, rgb, rgba, svg, transparent_black};
use jiff::Zoned;

use super::Elsewhere;
use super::drag::DragRow;
use super::tooltip::Tooltip;
use crate::clock::{self, Format};
use crate::theme;

/// A view preference the footer offers as an icon that is lit while on.
struct Toggle {
    id: &'static str,
    icon: &'static str,
    /// The icon's own size, since glyphs fill their box unevenly: a circle
    /// runs edge to edge where the signpost leaves a margin, so drawn equal
    /// it looks bigger.
    size: Pixels,
    /// The tooltip, while off and while on.
    tips: [&'static str; 2],
    /// A plain fn, so the element can outlive the borrow that built it.
    click: fn(&mut Elsewhere, &mut Context<Elsewhere>),
}

const FORMAT: Toggle = Toggle {
    id: "format",
    icon: "icons/clock-12.svg",
    size: px(14.),
    tips: ["show 12-hour time", "showing 12-hour time"],
    click: Elsewhere::toggle_format,
};

const WESTWARD: Toggle = Toggle {
    id: "westward",
    icon: "icons/milestone.svg",
    size: px(16.),
    tips: ["order west to east", "ordered west to east"],
    click: Elsewhere::toggle_westward,
};

impl Elsewhere {
    pub(super) fn unpin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pinned = None;
        self.close_editor(window, cx);
    }

    fn toggle_westward(&mut self, cx: &mut Context<Self>) {
        self.westward = !self.westward;
        cx.notify();
    }

    fn toggle_format(&mut self, cx: &mut Context<Self>) {
        self.format = match self.format {
            Format::TwentyFour => Format::Twelve,
            Format::Twelve => Format::TwentyFour,
        };
        cx.notify();
    }

    /// A footer toggle: an icon in the same box as the bin, lit in the accent
    /// while on, with a tooltip saying what it does.
    fn render_toggle(
        &self,
        toggle: &Toggle,
        on: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let Toggle {
            id,
            icon,
            size,
            tips,
            click,
        } = *toggle;
        div()
            .id(id)
            .w_7()
            .h_5()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            // An invisible border, so the icon sits in the same content box
            // as the bin's does inside its dashed one.
            .border_1()
            .border_color(transparent_black())
            .cursor_pointer()
            .hover(|style| style.bg(rgb(theme::SURFACE0)))
            .tooltip(Tooltip::text(tips[on as usize]))
            .on_click(cx.listener(move |this, _event, _window, cx| click(this, cx)))
            .child(
                svg()
                    .path(icon)
                    .size(size)
                    // An svg paints only in a color set on itself; the
                    // parent's does not reach it.
                    .text_color(if on {
                        rgb(theme::BLUE)
                    } else {
                        rgb(theme::OVERLAY0)
                    }),
            )
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
            .map(|zone| clock::reading(now, zone, self.format).time)
            .unwrap_or_else(|| clock::UNKNOWN.to_string());
        Some(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .items_center()
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

    /// Where a dragged row goes to be deleted. Dashed at rest so it reads as a
    /// place to drop rather than a button, lit in the accent once the row is
    /// over it.
    fn render_bin(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("bin")
            .w_7()
            .h_5()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .border_1()
            .border_dashed()
            .border_color(rgb(theme::OVERLAY0))
            .drag_over::<DragRow>(|style, _, _, _| {
                style
                    .border_color(rgb(theme::BLUE))
                    .bg(rgb(theme::SURFACE0))
            })
            .on_drop(
                cx.listener(|this, row: &DragRow, window, cx| {
                    this.delete(row.geonameid, window, cx)
                }),
            )
            .child(
                svg()
                    .path("icons/trash.svg")
                    .size_3p5()
                    // An svg paints only in a color set on itself; the
                    // parent's does not reach it.
                    .text_color(rgb(theme::SUBTEXT0))
                    .drag_over::<DragRow>(|style, _, _, _| style.text_color(rgb(theme::BLUE))),
            )
    }

    /// The strip along the bottom: a bin on the left for as long as a row is
    /// in flight, view preferences on the right. It is always there, at a
    /// fixed height, so the list above never shifts as controls come and go.
    pub(super) fn render_footer(&self, dragging: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .h_5()
            .text_xs()
            .child(div().children((dragging && self.drag.is_some()).then(|| self.render_bin(cx))))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_1()
                    .children(
                        (!self.saved.is_empty()).then(|| {
                            self.render_toggle(&FORMAT, self.format == Format::Twelve, cx)
                        }),
                    )
                    .children(
                        (self.saved.len() > 1)
                            .then(|| self.render_toggle(&WESTWARD, self.westward, cx)),
                    ),
            )
    }
}
