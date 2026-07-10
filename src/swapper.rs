use gpui::{Context, Window, div, prelude::*, rgb};
use rand::Rng;

pub struct Swapper {
    pub swap_started: bool,
    pub city_list: Vec<String>,
    pub city_index: usize,
}

impl Swapper {
    fn swap_city(&mut self) {
        let mut rng = rand::thread_rng();
        let city_count = self.city_list.len();
        let mut new_index = rng.gen_range(0..city_count);
        while new_index == self.city_index {
            new_index = rng.gen_range(0..city_count);
        }
        self.city_index = new_index;
    }
}

impl Render for Swapper {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.swap_started {
            div()
                .id("welcome")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(0x1e1e2e))
                .text_color(rgb(0xcdd6f4))
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.swap_started = true;
                    cx.notify();
                }))
                .child(format!("welcome"))
                .into_any_element()
        } else {
            div()
                .id("swapped")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(0x1e1e2e))
                .text_color(rgb(0xcdd6f4))
                .on_click(cx.listener(|this, _event, _window, cx| {
                    this.swap_city();
                    cx.notify();
                }))
                .child(self.city_list[self.city_index].clone())
                .into_any_element()
        }
    }
}
