use gpui::{
    App, Context, Div, Entity, Focusable, Pixels, SharedString, Window, div, prelude::*, px, rgb,
    rgba,
};
use jiff::Zoned;

use super::drag::{DragRow, drop_target};
use super::{Cancel, Commit, Elsewhere, Pin};
use crate::clock;
use crate::theme;
use crate::vendor::text_input::TextInput;

/// Width of the time column. Fixed so the reading and the what-if editor
/// occupy the same box, and so a row does not reflow as its digits change.
/// Sized to the format's longest reading in a 0.6em mono at 16px plus the
/// vendored input's 6px of padding either side, so the editor hugs the digits
/// rather than leaving slack on its right; every face on the mono list is
/// that width.
fn time_width(format: clock::Format) -> Pixels {
    px(format.glyphs() as f32 * 9.6 + 12.)
}

/// Height of the time column, and with it the row. This is the box the
/// vendored input renders at, reserved whether or not the editor is open, so
/// that opening one does not push the rest of the list down. The row is taller
/// than its text needs as a result; the honest fix is a shorter input.
const TIME_HEIGHT: Pixels = px(36.);

/// The row's left inset, so the label does not sit flush against the hover
/// wash's edge. The wash itself starts at the column edge, in line with the
/// search bar: it cannot hang out into the window padding, because the list
/// is a scroll container and clips to its own bounds on both axes, which
/// would square off the row's left corners.
const WASH: Pixels = px(8.);

/// The space either side of the rule between two rows.
const GAP: Pixels = px(6.);

/// Mono faces for the time column, best first. Equal-width digits keep a
/// reading from shifting as it ticks, which tabular figures would also do,
/// except that `tnum` is missing from several of the sans faces gpui falls
/// back to and there is no way to ask for the feature without naming a family
/// anyway.
const MONO: [&str; 8] = [
    "JetBrains Mono",
    "IBM Plex Mono",
    "SF Mono",
    "Menlo",
    "Cascadia Mono",
    "Adwaita Mono",
    "DejaVu Sans Mono",
    "Liberation Mono",
];

/// Settles on the time column's face once, at startup. gpui matches a family
/// name exactly and quietly drops back to the UI font when it misses, so the
/// list is checked against what the system actually carries. `None` leaves the
/// column proportional, which is no worse than not asking.
pub(super) fn mono(cx: &App) -> Option<SharedString> {
    let installed = cx.text_system().all_font_names();
    MONO.iter()
        .find(|family| installed.iter().any(|name| name == *family))
        .map(|family| SharedString::from(*family))
}

impl Elsewhere {
    pub(super) fn delete(&mut self, geonameid: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.saved.retain(|id| *id != geonameid);
        self.persist();
        if self
            .editing
            .as_ref()
            .is_some_and(|(id, _)| *id == geonameid)
        {
            self.close_editor(window, cx);
        }
        cx.notify();
    }

    /// Opens the what-if editor on a row's time. The current reading rides
    /// along as the placeholder, since the input cannot be prefilled.
    pub(super) fn edit(
        &mut self,
        geonameid: u32,
        current: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| TextInput::new(current, cx));
        window.focus(&input.focus_handle(cx));
        cx.observe(&input, Self::retime).detach();
        self.editing = Some((geonameid, input));
        cx.notify();
    }

    /// Re-pins on every keystroke in the editor that reads as a time, so the
    /// other rows follow along while the user types.
    fn retime(&mut self, input: Entity<TextInput>, cx: &mut Context<Self>) {
        let Some((geonameid, editor)) = &self.editing else {
            return;
        };
        if *editor != input {
            return;
        }
        if let Some(city) = self.index.city(*geonameid)
            && let Some(zone) = self.zones.get(&city.timezone)
            && let Some(pinned) = clock::pin(input.read(cx).text(), zone, &Zoned::now())
        {
            self.pinned = Some(Pin {
                anchor: Some(*geonameid),
                at: pinned,
            });
            cx.notify();
        }
    }

    /// Enter: the pin, if any, stays; the editor goes.
    fn commit(&mut self, _: &Commit, window: &mut Window, cx: &mut Context<Self>) {
        self.close_editor(window, cx);
    }

    /// Escape: back to the live clock.
    fn cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.pinned = None;
        self.close_editor(window, cx);
    }

    /// Drops the editor and hands focus back to the search input.
    pub(super) fn close_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing = None;
        window.focus(&self.picker.focus_handle(cx));
        cx.notify();
    }

    /// The what-if editor in the box the time column reserves for it. Enter
    /// and escape fall through the vendored input, so the wrapper carries the
    /// context that catches them. Shared with the map, which opens the same
    /// editor on a label's time.
    pub(super) fn render_editor(&self, input: &Entity<TextInput>, cx: &mut Context<Self>) -> Div {
        div()
            .key_context("PinEditor")
            .on_action(cx.listener(Self::commit))
            .on_action(cx.listener(Self::cancel))
            .w(time_width(self.format))
            .h(TIME_HEIGHT)
            .when_some(self.mono.clone(), Styled::font_family)
            .child(input.clone())
    }

    /// What the list shows while there is nothing in it: a hint, sat where the
    /// first row's label would be.
    pub(super) fn render_empty(&self) -> impl IntoElement + use<> {
        div()
            .h(TIME_HEIGHT)
            .flex()
            .items_center()
            .pl(WASH)
            .text_color(rgb(theme::OVERLAY0))
            .child("search for a city to start your list")
    }

    /// The `use<>` says the row borrows none of the arguments, which is what
    /// lets `render` build the list inside a closure that holds `cx`.
    pub(super) fn render_row(
        &self,
        geonameid: u32,
        now: &Zoned,
        dragging: bool,
        first: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let city = self.index.city(geonameid)?;
        let label = self.index.label(city);
        let reading = self
            .zones
            .get(&city.timezone)
            .map(|zone| clock::reading(now, zone, self.format));
        // Only a row whose zone resolved can anchor a pin.
        let known = reading.is_some();
        let (time, day) = match reading {
            Some(reading) => (reading.time, reading.day),
            None => (clock::UNKNOWN.to_string(), None),
        };
        // The rule that separates a row from the one above doubles as the
        // insertion line: it lights up, and a drop lands, only in the
        // hand-arranged order; the westward order does not reorder, but a row
        // can still be dragged out of it to the bin. The first row wears the
        // rule too, transparent, so a drop before it still has a line to show.
        // The same inset sits either side of every rule, and none at the
        // list's ends, where there is no rule to space from.
        let target = drop_target(div(), !self.westward)
            .when(!first, |row| row.border_color(rgb(theme::SURFACE0)))
            .py(GAP)
            .when(first, |row| row.pt_0())
            .when(last, |row| row.pb_0())
            .when(!self.westward, |row| {
                row.on_drop(cx.listener(move |this, row: &DragRow, _window, cx| {
                    this.drop(row.geonameid, Some(geonameid), cx)
                }))
            });
        Some(
            target.child(
                div()
                    .id(("row", geonameid as usize))
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .pl(WASH)
                    .rounded_md()
                    .hover(|style| style.bg(rgba(theme::WASH)))
                    // The whole row is the handle: there is no grip to find, and
                    // the cursor says so on hover.
                    .cursor_grab()
                    .on_drag(
                        DragRow {
                            geonameid,
                            label: label.clone(),
                        },
                        Self::drag_preview(cx),
                    )
                    .when(dragging && self.drag == Some(geonameid), |row| {
                        row.opacity(0.4)
                    })
                    // `flex_1` on its own is not enough: a flex item's automatic
                    // minimum is its own content, so a long label would hold the
                    // row open and shove the time and the x past the right edge.
                    // `min_w_0` lets the label give way instead, and truncate ends
                    // it in an ellipsis rather than wrapping out of the row.
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(rgb(theme::SUBTEXT0))
                            .child(label),
                    )
                    .children(
                        day.map(|day| div().text_xs().text_color(rgb(theme::OVERLAY0)).child(day)),
                    )
                    .child(match &self.editing {
                        Some((editing, input)) if *editing == geonameid => {
                            self.render_editor(input, cx).into_any_element()
                        }
                        // The cell is fixed so the column lines up; the pill inside
                        // wraps the digits, so the hover wash sits under them
                        // rather than under the empty half of a right-aligned cell.
                        // It runs the cell's full height, so the hover look is the
                        // same box the editor opens into.
                        _ => div()
                            .w(time_width(self.format))
                            .h(TIME_HEIGHT)
                            .flex()
                            .justify_end()
                            .child(
                                div()
                                    .id(("time", geonameid as usize))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    // The vendored input's inset, so the digits
                                    // hold still when the editor opens.
                                    .px(px(6.))
                                    .rounded_md()
                                    .when_some(self.mono.clone(), Styled::font_family)
                                    .when_some(
                                        self.pinned.as_ref().map(|pin| pin.accent().text),
                                        |cell, text| cell.text_color(rgb(text)),
                                    )
                                    .when(known, |cell| {
                                        cell.cursor_pointer()
                                            .hover(|style| style.bg(rgb(theme::SURFACE0)))
                                            .on_click(cx.listener({
                                                let time = time.clone();
                                                move |this, _event, window, cx| {
                                                    this.edit(geonameid, time.clone(), window, cx)
                                                }
                                            }))
                                    })
                                    .child(time.clone()),
                            )
                            .into_any_element(),
                    }),
            ),
        )
    }
}
