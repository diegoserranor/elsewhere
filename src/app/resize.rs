use gpui::{
    AnyElement, CursorStyle, Decorations, MouseButton, Pixels, ResizeEdge, Tiling, Window, div,
    prelude::*, px,
};

/// How far in from an edge the pointer can grab it. The window padding is
/// wider than this, so nothing clickable is ever covered.
const EDGE: Pixels = px(4.);
/// A corner's grab square: wider than an edge, so it is easy to hit.
const CORNER: Pixels = px(8.);

/// The invisible strips along the window's borders that resize it, standing
/// in for the ones the frame the window goes without would give. A maximized
/// window has none, and a tiled one none along its tiled sides.
pub(super) fn handles(window: &Window) -> Vec<AnyElement> {
    let tiling = match window.window_decorations() {
        Decorations::Client { tiling } if !window.is_maximized() => tiling,
        _ => return Vec::new(),
    };
    let Tiling {
        top,
        left,
        right,
        bottom,
    } = tiling;
    let free = |edge| match edge {
        ResizeEdge::Top => !top,
        ResizeEdge::Right => !right,
        ResizeEdge::Bottom => !bottom,
        ResizeEdge::Left => !left,
        ResizeEdge::TopLeft => !top && !left,
        ResizeEdge::TopRight => !top && !right,
        ResizeEdge::BottomRight => !bottom && !right,
        ResizeEdge::BottomLeft => !bottom && !left,
    };
    // The edges first, so the corners lie over their ends.
    [
        ResizeEdge::Top,
        ResizeEdge::Right,
        ResizeEdge::Bottom,
        ResizeEdge::Left,
        ResizeEdge::TopLeft,
        ResizeEdge::TopRight,
        ResizeEdge::BottomRight,
        ResizeEdge::BottomLeft,
    ]
    .into_iter()
    .filter(|edge| free(*edge))
    .map(|edge| handle(edge).into_any_element())
    .collect()
}

fn handle(edge: ResizeEdge) -> impl IntoElement {
    let strip = div().absolute();
    let strip = match edge {
        ResizeEdge::Top => strip.top_0().left(CORNER).right(CORNER).h(EDGE),
        ResizeEdge::Bottom => strip.bottom_0().left(CORNER).right(CORNER).h(EDGE),
        ResizeEdge::Left => strip.left_0().top(CORNER).bottom(CORNER).w(EDGE),
        ResizeEdge::Right => strip.right_0().top(CORNER).bottom(CORNER).w(EDGE),
        ResizeEdge::TopLeft => strip.top_0().left_0().size(CORNER),
        ResizeEdge::TopRight => strip.top_0().right_0().size(CORNER),
        ResizeEdge::BottomRight => strip.bottom_0().right_0().size(CORNER),
        ResizeEdge::BottomLeft => strip.bottom_0().left_0().size(CORNER),
    };
    let cursor = match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    };
    strip
        .cursor(cursor)
        .on_mouse_down(MouseButton::Left, move |_, window, _| {
            window.start_window_resize(edge)
        })
}
