use std::collections::HashMap;

use gpui::{Context, Entity, Focusable, Task, Window, div, prelude::*, rgb};
use jiff::Zoned;
use jiff::tz::TimeZone;

use crate::cities;
use crate::clock;
use crate::search::SearchIndex;
use crate::vendor::text_input::TextInput;

/// How many search results the picker offers at a time.
const RESULTS: usize = 8;

pub struct Elsewhere {
    input: Entity<TextInput>,
    index: SearchIndex,
    /// The last text seen in the input, to spot the keystrokes among the
    /// input's other notifications.
    query: String,
    /// The geonameids currently offered, best first.
    results: Vec<u32>,
    /// The geonameids picked so far, oldest first.
    saved: Vec<u32>,
    /// The zones of the saved cities, resolved once each. A zone this machine's
    /// tzdb does not know stays `None` rather than being retried every minute.
    zones: HashMap<String, Option<TimeZone>>,
    /// The clock, kept alive for as long as the window is.
    _tick: Task<()>,
}

impl Elsewhere {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new("search for a city...", cx));
        window.focus(&input.focus_handle(cx));
        cx.observe(&input, Self::search).detach();
        Self {
            input,
            index: SearchIndex::new(cities::load()),
            query: String::new(),
            results: Vec::new(),
            saved: Vec::new(),
            zones: HashMap::new(),
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
        }
        self.input.update(cx, |input, cx| input.reset(cx));
        self.query.clear();
        self.results.clear();
        cx.notify();
    }

    fn delete(&mut self, geonameid: u32, cx: &mut Context<Self>) {
        self.saved.retain(|id| *id != geonameid);
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
        // One reading of the clock for the whole pass, so the rows agree.
        let now = Zoned::now();
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(self.input.clone())
            .children(self.results.iter().filter_map(|geonameid| {
                let city = self.index.city(*geonameid)?;
                Some(
                    div()
                        .id(("result", *geonameid as usize))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(rgb(0x181825))
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0x313244)))
                        .active(|style| style.opacity(0.8))
                        .on_click(cx.listener({
                            let geonameid = *geonameid;
                            move |this, _event, _window, cx| this.save(geonameid, cx)
                        }))
                        .child(self.index.label(city)),
                )
            }))
            .children(self.saved.iter().filter_map(|geonameid| {
                let city = self.index.city(*geonameid)?;
                let reading = self
                    .zones
                    .get(&city.timezone)
                    .and_then(|zone| zone.as_ref())
                    .map(|zone| clock::reading(&now, zone));
                let (time, day) = match reading {
                    Some(reading) => (reading.time, reading.day),
                    None => (clock::UNKNOWN.to_string(), None),
                };
                Some(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .text_color(rgb(0xa6adc8))
                                .child(self.index.label(city)),
                        )
                        .children(
                            day.map(|day| div().text_xs().text_color(rgb(0x6c7086)).child(day)),
                        )
                        .child(div().child(time))
                        .child(
                            div()
                                .id(("delete", *geonameid as usize))
                                .px_2()
                                .rounded_md()
                                .text_color(rgb(0xf38ba8))
                                .cursor_pointer()
                                .hover(|style| style.bg(rgb(0x313244)))
                                .active(|style| style.opacity(0.8))
                                .on_click(cx.listener({
                                    let geonameid = *geonameid;
                                    move |this, _event, _window, cx| this.delete(geonameid, cx)
                                }))
                                .child("x"),
                        ),
                )
            }))
    }
}
