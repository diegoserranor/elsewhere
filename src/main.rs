use gpui::{
    App, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, prelude::*,
    px, size,
};

mod app;
mod assets;
mod cities;
mod clock;
mod geo;
mod hours;
mod land;
mod saved;
mod search;
mod theme;
mod vendor;
mod zones;

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

            // A tall shape, as the list it opens on is: ten rows or so, and a
            // map that scrolls sideways rather than shrinking to fit. Kept
            // under 600 so it still fits a 1200-pixel display at 2x.
            let bounds = Bounds::centered(None, size(px(400.), px(560.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // Narrower than this and a row cannot hold a name and a
                    // time; shorter and the search has no list under it.
                    window_min_size: Some(size(px(320.), px(360.))),
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
