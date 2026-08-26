use std::collections::HashMap;

use gpui::{Context, Entity, Focusable, ScrollHandle, Task, Window, actions, div, prelude::*, rgb};
use jiff::Zoned;
use jiff::tz::TimeZone;

mod drag;
mod row;
mod scrollbar;
mod search;
mod toolbar;

use crate::cities;
use crate::clock;
use crate::saved;
use crate::search::SearchIndex;
use crate::theme;
use crate::vendor::text_input::TextInput;

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
            .child(self.render_search(window, cx))
            .children(
                (self.saved.len() > 1 || self.pinned.is_some()).then(|| self.render_toolbar(cx)),
            )
            .children(
                order
                    .into_iter()
                    .filter_map(|geonameid| self.render_row(geonameid, &now, dragging, cx)),
            )
            .when(!self.westward && dragging && self.drag.is_some(), |list| {
                list.child(self.render_drop_tail(cx))
            })
    }
}
