use gpui::{Context, Entity, Focusable, Window, div, prelude::*, px, rgb};
use jiff::Zoned;

use super::drag::{DragRow, drop_target};
use super::{Cancel, Commit, Elsewhere};
use crate::clock;
use crate::saved;
use crate::theme;
use crate::vendor::text_input::TextInput;

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
            div()
                .group(group.clone())
                .flex()
                .flex_row()
                .gap_2()
                .items_center()
                .when(!self.westward, |row| {
                    // The border is always there, so an insertion line
                    // costs no layout.
                    drop_target(row).on_drop(cx.listener(
                        move |this, row: &DragRow, _window, cx| {
                            this.drop(row.geonameid, Some(geonameid), cx)
                        },
                    ))
                })
                .when(dragging && self.drag == Some(geonameid), |row| {
                    row.opacity(0.4)
                })
                .children((!self.westward).then(|| {
                    div()
                        .id(("grip", geonameid as usize))
                        .px_1()
                        .rounded_md()
                        .cursor_grab()
                        .text_color(rgb(theme::SUBTEXT0))
                        // Hidden at rest, dim once the row is hovered, and
                        // only full strength under the pointer itself.
                        .opacity(0.)
                        .group_hover(group.clone(), |style| style.opacity(0.5))
                        .hover(|style| style.opacity(1.).bg(rgb(theme::SURFACE0)))
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
                        .child("⠿")
                }))
                .child(div().flex_1().text_color(rgb(theme::SUBTEXT0)).child(label))
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
                        .w(px(64.))
                        .child(input.clone())
                        .into_any_element(),
                    _ => div()
                        .id(("time", geonameid as usize))
                        .px_1()
                        .rounded_md()
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
                        .child(time.clone())
                        .into_any_element(),
                })
                .child(
                    div()
                        .id(("delete", geonameid as usize))
                        .px_2()
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
