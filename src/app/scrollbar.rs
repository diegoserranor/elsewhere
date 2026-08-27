use gpui::{Bounds, Pixels, ScrollHandle, canvas, fill, point, prelude::*, px, rgb, size};

use crate::theme;

/// A scrollbar: a thumb painted only while the list overflows. It reads the
/// handle at paint time, after layout has settled, so it needs no state of its
/// own — but it is display-only; the wheel scrolls. `right` is the offset from
/// the container's right edge; negative hangs the thumb outside it, which is
/// how the saved list keeps its bar out of the delete column.
pub(super) fn scrollbar(handle: &ScrollHandle, right: Pixels) -> impl IntoElement {
    let scroll = handle.clone();
    canvas(
        move |_bounds, _window, _cx| scroll,
        move |bounds, scroll, window, _cx| {
            let overflow = scroll.max_offset().height;
            if overflow <= px(0.) {
                return;
            }
            let track = bounds.size.height;
            let viewport = scroll.bounds().size.height;
            let mut thumb = track * (viewport / (viewport + overflow));
            if thumb < px(20.) {
                thumb = px(20.);
            }
            let along = (track - thumb) * (-scroll.offset().y / overflow);
            window.paint_quad(
                fill(
                    Bounds::new(
                        point(bounds.origin.x, bounds.origin.y + along),
                        size(bounds.size.width, thumb),
                    ),
                    rgb(theme::SURFACE1),
                )
                .corner_radii(px(1.5)),
            );
        },
    )
    .absolute()
    .top_1()
    .bottom_1()
    .right(right)
    .w(px(3.))
}
