use std::rc::Rc;

use gpui::{
    Context, Entity, ScrollHandle, SharedString, Task, Window, actions, div, prelude::*, px, rgb,
};
use jiff::Zoned;

mod drag;
mod footer;
mod row;
mod scrollbar;
pub(crate) mod search;

use crate::cities;
use crate::clock;
use crate::saved;
use crate::search::SearchIndex;
use crate::theme;
use crate::vendor::text_input::TextInput;
use scrollbar::scrollbar;
use search::{SearchEvent, SearchPicker};

// Keys the pin editor answers to. The vendored input claims neither, so they
// bubble up to the wrapper div carrying the "PinEditor" context.
actions!(pin_editor, [Commit, Cancel]);

// Escape from anywhere in the window: back to the live clock. The search
// picker lets it through only when it has nothing of its own to clear.
actions!(elsewhere, [Unpin]);

/// A what-if time in effect.
pub(crate) struct Pin {
    /// The row the time was typed into, by geonameid.
    pub(crate) anchor: u32,
    /// The instant every row reads from, held in home's zone.
    pub(crate) at: Zoned,
}

pub struct Elsewhere {
    /// The search region, which owns its input and results and reports a choice
    /// back as a `SearchEvent`.
    picker: Entity<SearchPicker>,
    index: Rc<SearchIndex>,
    /// The geonameids picked so far, oldest first.
    saved: Vec<u32>,
    /// The zones of the saved cities.
    zones: clock::Zones,
    /// The geonameid a drag is carrying, while one is in flight.
    drag: Option<u32>,
    /// Whether the list is shown ordered by longitude rather than by hand. A
    /// view preference, deliberately not persisted.
    westward: bool,
    /// The what-if time in effect. `None` means the rows follow the real clock.
    pinned: Option<Pin>,
    /// The row whose time is being typed over, and the input doing it.
    editing: Option<(u32, Entity<TextInput>)>,
    /// Where the saved list has scrolled to. Doubles as the list's on-screen
    /// bounds, which the scrollbar reads.
    saved_scroll: ScrollHandle,
    /// The mono face the time column reads in, if the system has one.
    mono: Option<SharedString>,
    /// The clock, kept alive for as long as the window is.
    _tick: Task<()>,
}

impl Elsewhere {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let index = Rc::new(SearchIndex::new(cities::load()));
        let picker = cx.new(|cx| SearchPicker::new(index.clone(), window, cx));
        cx.subscribe(&picker, Self::picked).detach();

        let mut saved = saved::load();
        // A regenerated dataset may have dropped a city saved by an older run.
        saved.retain(|id| index.city(*id).is_some());
        // Resolved up front, so rows restored from disk show a time straight away.
        let mut zones = clock::Zones::new();
        for geonameid in &saved {
            if let Some(city) = index.city(*geonameid) {
                zones.resolve(&city.timezone);
            }
        }

        Self {
            picker,
            index,
            saved,
            zones,
            drag: None,
            westward: false,
            pinned: None,
            editing: None,
            saved_scroll: ScrollHandle::new(),
            mono: row::mono(cx),
            _tick: tick(cx),
        }
    }

    fn picked(
        &mut self,
        _picker: Entity<SearchPicker>,
        event: &SearchEvent,
        cx: &mut Context<Self>,
    ) {
        let SearchEvent::Picked(geonameid) = event;
        self.save(*geonameid, cx);
    }

    fn save(&mut self, geonameid: u32, cx: &mut Context<Self>) {
        if !self.saved.contains(&geonameid) {
            self.saved.push(geonameid);
            if let Some(city) = self.index.city(geonameid) {
                self.zones.resolve(&city.timezone);
            }
            saved::save(&self.saved);
            // Only the hand-arranged order appends at the end; westward drops
            // the new row wherever its longitude falls.
            if !self.westward {
                self.saved_scroll.scroll_to_bottom();
            }
        }
        cx.notify();
    }
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // gpui refreshes the window once a drag ends, however it ended, so this
        // is where a cancelled drag is forgotten.
        let dragging = cx.has_active_drag();
        if !dragging {
            self.drag = None;
        }

        // One reading of the clock for the whole pass, so the rows agree. A
        // pinned instant stands in for the clock wholesale, which is the whole
        // feature: every row simply renders that moment instead of this one.
        let now = self
            .pinned
            .as_ref()
            .map_or_else(Zoned::now, |pin| pin.at.clone());
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
            .key_context("Elsewhere")
            .on_action(cx.listener(|this, _: &Unpin, window, cx| this.unpin(window, cx)))
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(rgb(theme::BASE))
            .text_color(rgb(theme::TEXT))
            // Both are inset by a row's gutter on the right, so the search bar
            // and the footer end where the times do. The list between keeps
            // the full width: its gutter is the delete column.
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mr(row::GUTTER)
                    .child(self.picker.clone()),
            )
            .children(self.render_banner(&now, cx))
            // The rows scroll on their own, so the input and toolbar above stay
            // put however long the list grows. `min_h_0` is what lets this flex
            // child shrink below its content and actually overflow.
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("saved")
                            .size_full()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .overflow_y_scroll()
                            .track_scroll(&self.saved_scroll)
                            .children(order.into_iter().filter_map(|geonameid| {
                                self.render_row(geonameid, &now, dragging, cx)
                            }))
                            .when(!self.westward && dragging && self.drag.is_some(), |list| {
                                list.child(self.render_drop_tail(cx))
                            }),
                    )
                    // Hung out in the window padding, clear of the delete
                    // column the rows end in.
                    .child(scrollbar(&self.saved_scroll, px(-6.))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mr(row::GUTTER)
                    .child(self.render_footer(dragging, cx)),
            )
    }
}
