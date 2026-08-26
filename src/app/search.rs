use std::rc::Rc;

use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, MouseDownEvent, ScrollHandle,
    Window, deferred, div, prelude::*, px, rgb,
};

use super::scrollbar::scrollbar;
use crate::search::SearchIndex;
use crate::theme;
use crate::vendor::text_input::TextInput;

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
        cx.notify();
    }

    /// Closes the results panel. `query` stays as it is: `search()` re-runs only
    /// when the input text differs from it, so clearing it would let the next
    /// input notification reopen the panel.
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.results.clear();
        cx.notify();
    }

    /// Hands the choice up and starts over on an empty input.
    fn pick(&mut self, geonameid: u32, cx: &mut Context<Self>) {
        self.input.update(cx, |input, cx| input.reset(cx));
        self.query.clear();
        self.results.clear();
        cx.emit(SearchEvent::Picked(geonameid));
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
                        .child(scrollbar(&self.scroll))
                        .child(
                            div()
                                .id("results")
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .max_h(window.viewport_size().height - px(80.))
                                .overflow_y_scroll()
                                .track_scroll(&self.scroll)
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
                                                    this.pick(geonameid, cx)
                                                }
                                            }))
                                            .child(self.index.label(city)),
                                    )
                                })),
                        ),
                )
            }))
    }
}
