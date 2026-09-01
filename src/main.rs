use gpui::{
    App, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, prelude::*,
    px, size,
};

mod app;
mod assets;
mod cities;
mod clock;
mod saved;
mod search;
mod theme;
mod vendor;

fn main() {
    Application::new()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            vendor::text_input::register_key_bindings(cx);
            cx.bind_keys([
                KeyBinding::new("enter", app::Commit, Some("PinEditor")),
                KeyBinding::new("escape", app::Cancel, Some("PinEditor")),
                KeyBinding::new("enter", app::search::Confirm, Some("SearchPicker")),
                KeyBinding::new("escape", app::search::Clear, Some("SearchPicker")),
                KeyBinding::new("up", app::search::MoveUp, Some("SearchPicker")),
                KeyBinding::new("down", app::search::MoveDown, Some("SearchPicker")),
                KeyBinding::new("escape", app::Unpin, Some("Elsewhere")),
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
