use gpui::{
    App, Context, Entity, Focusable, Pixels, SharedString, Window, div, prelude::*, px, rgb, rgba,
};
use jiff::Zoned;

use super::drag::{DragRow, drop_target};
use super::{Cancel, Commit, Elsewhere};
use crate::clock;
use crate::saved;
use crate::theme;
use crate::vendor::text_input::TextInput;

/// Width of the time column. Fixed so the reading and the what-if editor
/// occupy the same box, and so a row does not reflow as its digits change.
const TIME_WIDTH: Pixels = px(64.);

/// Height of the time column, and with it the row. This is the box the
/// vendored input renders at, reserved whether or not the editor is open, so
/// that opening one does not push the rest of the list down. The row is taller
/// than its text needs as a result; the honest fix is a shorter input.
const TIME_HEIGHT: Pixels = px(36.);

/// Width of the cell the delete x sits in. The x is invisible at rest but its
/// cell is not, so the times keep a column of their own whatever the pointer
/// is doing.
const CONTROL_WIDTH: Pixels = px(24.);

/// How far the search bar and the footer are inset on the right to meet that
/// column: the x's cell plus the row's `gap_2` beside it. Spelled out because
/// `Pixels` does not add in a const.
pub(super) const GUTTER: Pixels = px(32.);

/// How far a row's hover wash reaches into the window padding, so the label
/// does not sit flush against the wash's edge.
const WASH: Pixels = px(8.);

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
    fn delete(&mut self, geonameid: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.saved.retain(|id| *id != geonameid);
        saved::save(&self.saved);
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
    fn edit(
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
            self.pinned = Some(pinned);
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

    /// The `use<>` says the row borrows none of the arguments, which is what
    /// lets `render` build the list inside a closure that holds `cx`.
    pub(super) fn render_row(
        &self,
        geonameid: u32,
        now: &Zoned,
        dragging: bool,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let city = self.index.city(geonameid)?;
        let label = self.index.label(city);
        let reading = self
            .zones
            .get(&city.timezone)
            .map(|zone| clock::reading(now, zone));
        // Only a row whose zone resolved can anchor a pin.
        let known = reading.is_some();
        let (time, day) = match reading {
            Some(reading) => (reading.time, reading.day),
            None => (clock::UNKNOWN.to_string(), None),
        };
        let group = format!("saved-{geonameid}");
        Some(
            // The transparent border is dressed on in either view, so an
            // insertion line costs no layout and toggling the order does not
            // lift the rows by its width. Only the drag itself is withheld
            // from the westward order, which does not reorder.
            drop_target(div())
                .id(("row", geonameid as usize))
                .group(group.clone())
                .flex()
                .flex_row()
                .gap_2()
                .items_center()
                .ml(-WASH)
                .pl(WASH)
                .rounded_md()
                .hover(|style| style.bg(rgba(theme::WASH)))
                .when(!self.westward, |row| {
                    // The whole row is the handle: there is no grip to find,
                    // and the cursor says so on hover.
                    row.cursor_grab()
                        .on_drag(
                            DragRow {
                                geonameid,
                                label: label.clone(),
                            },
                            {
                                let this = cx.weak_entity();
                                move |row: &DragRow, _position, _window, cx| {
                                    this.update(cx, |this, _cx| this.drag = Some(row.geonameid))
                                        .ok();
                                    cx.new(|_cx| DragRow {
                                        geonameid: row.geonameid,
                                        label: row.label.clone(),
                                    })
                                }
                            },
                        )
                        .on_drop(cx.listener(move |this, row: &DragRow, _window, cx| {
                            this.drop(row.geonameid, Some(geonameid), cx)
                        }))
                })
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
                    Some((editing, input)) if *editing == geonameid => div()
                        // Enter and escape fall through the input and
                        // land on this wrapper's context.
                        .key_context("PinEditor")
                        .on_action(cx.listener(Self::commit))
                        .on_action(cx.listener(Self::cancel))
                        .w(TIME_WIDTH)
                        .h(TIME_HEIGHT)
                        .when_some(self.mono.clone(), Styled::font_family)
                        .child(input.clone())
                        .into_any_element(),
                    // The cell is fixed so the column lines up; the pill inside
                    // wraps the digits, so the hover wash sits under them
                    // rather than under the empty half of a right-aligned cell.
                    _ => div()
                        .w(TIME_WIDTH)
                        .h(TIME_HEIGHT)
                        .flex()
                        .items_center()
                        .justify_end()
                        .child(
                            div()
                                .id(("time", geonameid as usize))
                                .px_1()
                                .rounded_md()
                                .when_some(self.mono.clone(), Styled::font_family)
                                .when(self.pinned.is_some(), |cell| {
                                    cell.text_color(rgb(theme::YELLOW))
                                })
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
                })
                .child(
                    div()
                        .id(("delete", geonameid as usize))
                        .w(CONTROL_WIDTH)
                        .flex()
                        .justify_center()
                        .rounded_md()
                        .text_color(rgb(theme::RED))
                        .opacity(0.)
                        .group_hover(group, |style| style.opacity(0.5))
                        .cursor_pointer()
                        .hover(|style| style.opacity(1.).bg(rgb(theme::SURFACE0)))
                        .active(|style| style.opacity(0.8))
                        .on_click(cx.listener(move |this, _event, window, cx| {
                            this.delete(geonameid, window, cx)
                        }))
                        .child("x"),
                ),
        )
    }
}
