use gpui::{Context, Entity, Focusable, Window, div, prelude::*, rgb};

use crate::vendor::text_input::TextInput;

pub struct Saver {
    input: Entity<TextInput>,
    saved: Vec<String>,
}

impl Saver {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new("type something...", cx));
        window.focus(&input.focus_handle(cx));
        Self {
            input,
            saved: Vec::new(),
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).text().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.saved.push(text);
        self.input.update(cx, |input, cx| input.reset(cx));
        cx.notify();
    }

    fn delete(&mut self, index: usize, cx: &mut Context<Self>) {
        self.saved.remove(index);
        cx.notify();
    }
}

impl Render for Saver {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().child(self.input.clone()))
                    .child(
                        div()
                            .id("save")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(rgb(0x89b4fa))
                            .text_color(rgb(0x1e1e2e))
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0xb4befe)))
                            .active(|style| style.opacity(0.8))
                            .on_click(cx.listener(|this, _event, _window, cx| this.save(cx)))
                            .child("Save"),
                    ),
            )
            .children(self.saved.iter().enumerate().rev().map(|(index, text)| {
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .items_center()
                    .child(div().flex_1().text_color(rgb(0xa6adc8)).child(text.clone()))
                    .child(
                        div()
                            .id(("delete", index))
                            .px_2()
                            .rounded_md()
                            .text_color(rgb(0xf38ba8))
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x313244)))
                            .active(|style| style.opacity(0.8))
                            .on_click(
                                cx.listener(move |this, _event, _window, cx| {
                                    this.delete(index, cx)
                                }),
                            )
                            .child("x"),
                    )
            }))
    }
}
