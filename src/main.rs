use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};

mod swapper;

fn main() {
    Application::new().run(|cx: &mut App| {
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
            |_window, cx| {
                cx.new(|_| swapper::Swapper {
                    swap_started: false,
                    city_list: vec![
                        "guayaquil".to_string(),
                        "indianapolis".to_string(),
                        "portland".to_string(),
                        "barcelona".to_string(),
                        "london".to_string(),
                        "hamburg".to_string(),
                        "seoul".to_string(),
                        "sidney".to_string(),
                        "hyderabad".to_string(),
                        "abuja".to_string(),
                        "tokyo".to_string(),
                    ],
                    city_index: 0,
                })
            },
        )
        .expect("failed to open window");
        cx.activate(true);
    });
}
