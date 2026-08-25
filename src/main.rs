use gpui::{
    App, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, prelude::*,
    px, size,
};

mod app;
mod cities;
mod clock;
mod saved;
mod search;
mod vendor;

fn main() {
    Application::new().run(|cx: &mut App| {
        vendor::text_input::register_key_bindings(cx);
        cx.bind_keys([
            KeyBinding::new("enter", app::Commit, Some("PinEditor")),
            KeyBinding::new("escape", app::Cancel, Some("PinEditor")),
        ]);

        let bounds = Bounds::centered(None, size(px(400.), px(300.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("elsewhere".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| app::Elsewhere::new(window, cx)),
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
