use std::collections::HashMap;

use gpui::{
    Bounds, Context, Entity, Focusable, MouseDownEvent, ScrollHandle, Task, Window, actions,
    canvas, deferred, div, fill, point, prelude::*, px, rgb, size,
};
use jiff::Zoned;
use jiff::tz::TimeZone;

use crate::cities;
use crate::clock;
use crate::saved;
use crate::search::SearchIndex;
use crate::theme;
use crate::vendor::text_input::TextInput;

/// How many search results the picker offers at a time.
const RESULTS: usize = 8;

// Keys the pin editor answers to. The vendored input claims neither, so they
// bubble up to the wrapper div carrying the "PinEditor" context.
actions!(pin_editor, [Commit, Cancel]);

pub struct Elsewhere {
    input: Entity<TextInput>,
    index: SearchIndex,
    /// The last text seen in the input, to spot the keystrokes among the
    /// input's other notifications.
    query: String,
    /// The geonameids currently offered, best first.
    results: Vec<u32>,
    /// Where the results panel has scrolled to. Doubles as the panel's on-screen
    /// bounds, which the dismiss guard and the scrollbar both read.
    results_scroll: ScrollHandle,
    /// The geonameids picked so far, oldest first.
    saved: Vec<u32>,
    /// The zones of the saved cities, resolved once each. A zone this machine's
    /// tzdb does not know stays `None` rather than being retried every minute.
    zones: HashMap<String, Option<TimeZone>>,
    /// The geonameid a drag is carrying, while one is in flight.
    drag: Option<u32>,
    /// Whether the list is shown ordered by longitude rather than by hand. A
    /// view preference, deliberately not persisted.
    westward: bool,
    /// The instant every row reads from while a what-if time is set, held in
    /// home's zone. `None` means the rows follow the real clock.
    pinned: Option<Zoned>,
    /// The row whose time is being typed over, and the input doing it.
    editing: Option<(u32, Entity<TextInput>)>,
    /// The clock, kept alive for as long as the window is.
    _tick: Task<()>,
}

/// The row that follows the cursor during a drag.
struct DragRow {
    geonameid: u32,
    label: String,
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

impl Elsewhere {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new("search for a city...", cx));
        window.focus(&input.focus_handle(cx));
        cx.observe(&input, Self::search).detach();

        let index = SearchIndex::new(cities::load());
        let mut saved = saved::load();
        // A regenerated dataset may have dropped a city saved by an older run.
        saved.retain(|id| index.city(*id).is_some());
        let zones = zones_for(&index, &saved);

        Self {
            input,
            index,
            query: String::new(),
            results: Vec::new(),
            results_scroll: ScrollHandle::new(),
            saved,
            zones,
            drag: None,
            westward: false,
            pinned: None,
            editing: None,
            _tick: tick(cx),
        }
    }

    /// The input emits no change event, so this runs on every notification it
    /// sends — cursor moves and selections included — and re-searches only when
    /// the text itself changed.
    fn search(&mut self, input: Entity<TextInput>, cx: &mut Context<Self>) {
        if input.read(cx).text() == self.query {
            return;
        }
        self.query = input.read(cx).text().to_string();
        self.results = self
            .index
            .search(&self.query, RESULTS)
            .iter()
            .map(|city| city.geonameid)
            .collect();
        cx.notify();
    }

    fn save(&mut self, geonameid: u32, cx: &mut Context<Self>) {
        if !self.saved.contains(&geonameid) {
            self.saved.push(geonameid);
            if let Some(city) = self.index.city(geonameid)
                && !self.zones.contains_key(&city.timezone)
            {
                let zone = TimeZone::get(&city.timezone).ok();
                self.zones.insert(city.timezone.clone(), zone);
            }
            saved::save(&self.saved);
        }
        self.input.update(cx, |input, cx| input.reset(cx));
        self.query.clear();
        self.results.clear();
        cx.notify();
    }

    /// Closes the results panel. `query` stays as it is: `search()` re-runs only
    /// when the input text differs from it, so clearing it would let the next
    /// input notification reopen the panel.
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.results.clear();
        cx.notify();
    }

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
            && let Some(Some(zone)) = self.zones.get(&city.timezone)
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

    fn unpin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pinned = None;
        self.close_editor(window, cx);
    }

    /// Drops the editor and hands focus back to the search input.
    fn close_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing = None;
        window.focus(&self.input.focus_handle(cx));
        cx.notify();
    }

    fn toggle_westward(&mut self, cx: &mut Context<Self>) {
        self.westward = !self.westward;
        cx.notify();
    }

    /// Lands a dragged row before `before`, or at the end without one.
    fn drop(&mut self, dragged: u32, before: Option<u32>, cx: &mut Context<Self>) {
        self.drag = None;
        if saved::reorder(&mut self.saved, dragged, before) {
            saved::save(&self.saved);
        }
        cx.notify();
    }
}

/// The zones of `saved`, resolved once each, so rows restored from disk show a
/// time straight away.
fn zones_for(index: &SearchIndex, saved: &[u32]) -> HashMap<String, Option<TimeZone>> {
    let mut zones = HashMap::new();
    for geonameid in saved {
        if let Some(city) = index.city(*geonameid)
            && !zones.contains_key(&city.timezone)
        {
            zones.insert(city.timezone.clone(), TimeZone::get(&city.timezone).ok());
        }
    }
    zones
}

/// The results panel's scrollbar: a thumb painted only while the list
/// overflows. It reads the handle at paint time, after layout has settled, so
/// it needs no state of its own — but it is display-only; the wheel scrolls.
fn scrollbar(handle: &ScrollHandle) -> impl IntoElement {
    let scroll = handle.clone();
    canvas(
        move |_bounds, _window, _cx| scroll,
        move |bounds, scroll, window, _cx| {
            let overflow = scroll.max_offset().height;
            if overflow <= px(0.) {
                return;
            }
            let track = bounds.size.height;
            let viewport = scroll.bounds().size.height;
            let mut thumb = track * (viewport / (viewport + overflow));
            if thumb < px(20.) {
                thumb = px(20.);
            }
            let along = (track - thumb) * (-scroll.offset().y / overflow);
            window.paint_quad(
                fill(
                    Bounds::new(
                        point(bounds.origin.x, bounds.origin.y + along),
                        size(bounds.size.width, thumb),
                    ),
                    rgb(theme::SURFACE1),
                )
                .corner_radii(px(1.5)),
            );
        },
    )
    .absolute()
    .top_1()
    .bottom_1()
    .right_0p5()
    .w(px(3.))
}

/// Re-renders on every minute boundary, so the shown times stay honest without
/// waking the app up in between.
fn tick(cx: &mut Context<Elsewhere>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor()
                .timer(clock::until_next_minute(&Zoned::now()))
                .await;
            if this.update(cx, |_, cx| cx.notify()).is_err() {
                // The window is gone.
                break;
            }
        }
    })
}

impl Render for Elsewhere {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // gpui refreshes the window once a drag ends, however it ended, so this
        // is where a cancelled drag is forgotten.
        let dragging = cx.has_active_drag();
        if !dragging {
            self.drag = None;
        }

        // One reading of the clock for the whole pass, so the rows agree. A
        // pinned instant stands in for the clock wholesale, which is the whole
        // feature: every row simply renders that moment instead of this one.
        let now = self.pinned.clone().unwrap_or_else(Zoned::now);
        // The westward view is derived here and nowhere else: the stored order
        // stays the one the user arranged by hand.
        let order: Vec<u32> = if self.westward {
            saved::westward(
                self.saved
                    .iter()
                    .filter_map(|geonameid| {
                        let city = self.index.city(*geonameid)?;
                        Some((*geonameid, city.longitude))
                    })
                    .collect(),
            )
        } else {
            self.saved.clone()
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(rgb(theme::BASE))
            .text_color(rgb(theme::TEXT))
            .child(
                // The results float over the saved list rather than sitting in
                // the column, so the rows below hold still while the user types.
                div().relative().child(self.input.clone()).children(
                    (!self.results.is_empty()).then(|| {
                        deferred(
                            div()
                                .occlude()
                                .absolute()
                                .top_full()
                                .left_0()
                                .right_0()
                                .mt_1()
                                .p_1()
                                .rounded_md()
                                .bg(rgb(theme::MANTLE))
                                .border_1()
                                .border_color(rgb(theme::SURFACE0))
                                .shadow_lg()
                                .on_mouse_down_out(cx.listener(
                                    |this, event: &MouseDownEvent, _window, cx| {
                                        // Everything above the panel is the input
                                        // strip; a click there keeps the panel open.
                                        if event.position.y < this.results_scroll.bounds().top() {
                                            return;
                                        }
                                        this.dismiss(cx)
                                    },
                                ))
                                .child(scrollbar(&self.results_scroll))
                                .child(
                                    div()
                                        .id("results")
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .max_h(window.viewport_size().height - px(80.))
                                        .overflow_y_scroll()
                                        .track_scroll(&self.results_scroll)
                                        .children(self.results.iter().filter_map(|geonameid| {
                                            let city = self.index.city(*geonameid)?;
                                            Some(
                                                div()
                                                    .id(("result", *geonameid as usize))
                                                    .px_2()
                                                    .py_1()
                                                    .rounded_md()
                                                    .cursor_pointer()
                                                    .hover(|style| style.bg(rgb(theme::SURFACE0)))
                                                    .active(|style| style.opacity(0.8))
                                                    .on_click(cx.listener({
                                                        let geonameid = *geonameid;
                                                        move |this, _event, _window, cx| {
                                                            this.save(geonameid, cx)
                                                        }
                                                    }))
                                                    .child(self.index.label(city)),
                                            )
                                        })),
                                ),
                        )
                    }),
                ),
            )
            .children((self.saved.len() > 1 || self.pinned.is_some()).then(|| {
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .children(self.pinned.is_some().then(|| {
                        div()
                            .id("unpin")
                            .px_2()
                            .text_xs()
                            .rounded_md()
                            .cursor_pointer()
                            .text_color(rgb(theme::YELLOW))
                            .hover(|style| style.bg(rgb(theme::SURFACE0)))
                            .on_click(
                                cx.listener(|this, _event, window, cx| this.unpin(window, cx)),
                            )
                            .child("back to now")
                    }))
                    .children((self.saved.len() > 1).then(|| {
                        div()
                            .id("westward")
                            .px_2()
                            .text_xs()
                            .rounded_md()
                            .cursor_pointer()
                            .text_color(if self.westward {
                                rgb(theme::BLUE)
                            } else {
                                rgb(theme::OVERLAY0)
                            })
                            .hover(|style| style.bg(rgb(theme::SURFACE0)))
                            .on_click(
                                cx.listener(|this, _event, _window, cx| this.toggle_westward(cx)),
                            )
                            .child("west → east")
                    }))
            }))
            .children(order.into_iter().filter_map(|geonameid| {
                let city = self.index.city(geonameid)?;
                let label = self.index.label(city);
                let reading = self
                    .zones
                    .get(&city.timezone)
                    .and_then(|zone| zone.as_ref())
                    .map(|zone| clock::reading(&now, zone));
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
                            row.border_t_2()
                                .border_color(gpui::transparent_black())
                                .drag_over::<DragRow>(|style, _, _, _| {
                                    style.border_color(rgb(theme::BLUE))
                                })
                                .on_drop(cx.listener(move |this, row: &DragRow, _window, cx| {
                                    this.drop(row.geonameid, Some(geonameid), cx)
                                }))
                        })
                        .when(dragging && self.drag == Some(geonameid), |row| {
                            row.opacity(0.4)
                        })
                        .children((!self.westward).then(|| {
                            div()
                                .id(("grip", geonameid as usize))
                                .cursor_grab()
                                .text_color(rgb(theme::SUBTEXT0))
                                .opacity(0.4)
                                .group_hover(group, |style| style.opacity(1.))
                                .on_drag(
                                    DragRow {
                                        geonameid,
                                        label: label.clone(),
                                    },
                                    {
                                        let this = cx.weak_entity();
                                        move |row: &DragRow, _position, _window, cx| {
                                            this.update(cx, |this, _cx| {
                                                this.drag = Some(row.geonameid)
                                            })
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
                            day.map(|day| {
                                div().text_xs().text_color(rgb(theme::OVERLAY0)).child(day)
                            }),
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
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(theme::SURFACE0)))
                                .active(|style| style.opacity(0.8))
                                .on_click(cx.listener(move |this, _event, window, cx| {
                                    this.delete(geonameid, window, cx)
                                }))
                                .child("x"),
                        ),
                )
            }))
            // The space below the list catches a drop meant for the end.
            .when(!self.westward && dragging && self.drag.is_some(), |list| {
                list.child(
                    div()
                        .flex_1()
                        .min_h_4()
                        .border_t_2()
                        .border_color(gpui::transparent_black())
                        .drag_over::<DragRow>(|style, _, _, _| style.border_color(rgb(theme::BLUE)))
                        .on_drop(cx.listener(|this, row: &DragRow, _window, cx| {
                            this.drop(row.geonameid, None, cx)
                        })),
                )
            })
    }
}
