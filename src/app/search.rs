use std::rc::Rc;

use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseDownEvent, ScrollHandle,
    Window, actions, deferred, div, prelude::*, px, rgb,
};

use super::scrollbar::scrollbar;
use crate::search::SearchIndex;
use crate::theme;
use crate::vendor::text_input::TextInput;

// Keys the picker answers to. The vendored input claims none of them, so they
// bubble up to the wrapper div carrying the "SearchPicker" context.
actions!(search, [Confirm, Clear, MoveUp, MoveDown]);

/// How many search results the picker offers at a time.
pub(super) const RESULTS: usize = 8;

pub(super) struct SearchPicker {
    input: Entity<TextInput>,
    index: Rc<SearchIndex>,
    /// The last text seen in the input, to spot the keystrokes among the
    /// input's other notifications.
    query: String,
    /// The geonameids currently offered, best first.
    results: Vec<u32>,
    /// Which result is highlighted, as an index into `results`. Only meaningful
    /// while `results` is non-empty.
    selected: usize,
    /// Where the results panel has scrolled to. Doubles as the panel's on-screen
    /// bounds, which the dismiss guard and the scrollbar both read.
    scroll: ScrollHandle,
}

pub(super) enum SearchEvent {
    /// The user chose a city; the picker has already reset itself.
    Picked(u32),
}

impl EventEmitter<SearchEvent> for SearchPicker {}

impl SearchPicker {
    pub(super) fn new(index: Rc<SearchIndex>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new("search for a city...", cx));
        window.focus(&input.focus_handle(cx));
        cx.observe(&input, Self::search).detach();

        Self {
            input,
            index,
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            scroll: ScrollHandle::new(),
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
        // Every keystroke reshuffles the results, so the highlight goes back to
        // the top hit: plain Enter saves the best match.
        self.selected = 0;
        cx.notify();
    }

    /// Closes the results panel. `query` stays as it is: `search()` re-runs only
    /// when the input text differs from it, so clearing it would let the next
    /// input notification reopen the panel.
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.results.clear();
        cx.notify();
    }

    /// Back to an empty input and no results. `query` is cleared alongside the
    /// input text so the two stay in step and `search()` stays quiet.
    fn reset(&mut self, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.reset(cx));
        self.query.clear();
        self.results.clear();
        self.selected = 0;
    }

    /// Hands the choice up and starts over on an empty input.
    fn pick(&mut self, geonameid: u32, cx: &mut Context<Self>) {
        self.reset(cx);
        cx.emit(SearchEvent::Picked(geonameid));
        cx.notify();
    }

    fn confirm(&mut self, _: &Confirm, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(&geonameid) = self.results.get(self.selected) {
            self.pick(geonameid, cx);
        }
    }

    fn clear(&mut self, _: &Clear, _window: &mut Window, cx: &mut Context<Self>) {
        // Nothing here to clear: let the window have the key instead.
        if self.query.is_empty() && self.results.is_empty() {
            cx.propagate();
            return;
        }
        self.reset(cx);
        cx.notify();
    }

    fn move_up(&mut self, _: &MoveUp, _window: &mut Window, cx: &mut Context<Self>) {
        if self.results.is_empty() {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }

    fn move_down(&mut self, _: &MoveDown, _window: &mut Window, cx: &mut Context<Self>) {
        if self.results.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.results.len() - 1);
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }
}

impl Focusable for SearchPicker {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.focus_handle(cx)
    }
}

impl Render for SearchPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The results float over the saved list rather than sitting in
        // the column, so the rows below hold still while the user types.
        div()
            .relative()
            .key_context("SearchPicker")
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::clear))
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
            .child(self.input.clone())
            .children((!self.results.is_empty()).then(|| {
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
                                if event.position.y < this.scroll.bounds().top() {
                                    return;
                                }
                                this.dismiss(cx)
                            },
                        ))
                        .child(scrollbar(&self.scroll, px(2.)))
                        .child(
                            div()
                                .id("results")
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .max_h(window.viewport_size().height - px(80.))
                                .overflow_y_scroll()
                                .track_scroll(&self.scroll)
                                .children(self.results.iter().enumerate().filter_map(
                                    |(i, geonameid)| {
                                        let city = self.index.city(*geonameid)?;
                                        Some(
                                            div()
                                                .id(("result", *geonameid as usize))
                                                .px_2()
                                                .py_1()
                                                .rounded_md()
                                                // One line per result, however
                                                // long the label runs.
                                                .truncate()
                                                .cursor_pointer()
                                                // The keyboard highlight wears
                                                // the hover look, held on.
                                                .when(i == self.selected, |row| {
                                                    row.bg(rgb(theme::SURFACE0))
                                                })
                                                .hover(|style| style.bg(rgb(theme::SURFACE0)))
                                                .active(|style| style.opacity(0.8))
                                                .on_click(cx.listener({
                                                    let geonameid = *geonameid;
                                                    move |this, _event, _window, cx| {
                                                        this.pick(geonameid, cx)
                                                    }
                                                }))
                                                .child(self.index.label(city)),
                                        )
                                    },
                                )),
                        ),
                )
            }))
    }
}
