//! The map view: the world painted in one canvas, land first, then the night
//! it is turning through, with the saved cities laid over it as divs.
mod labels;
mod pins;

use gpui::{
    Bounds, Context, Path, PathBuilder, Pixels, Point, ScrollDelta, ScrollWheelEvent, Size, Window,
    canvas, div, fill, point, prelude::*, px, rgb, rgba, size,
};
use jiff::{RoundMode, Span, Unit, Zoned, ZonedRound};

use super::{Elsewhere, Pin};
use crate::geo::{self, LonLat};
use crate::theme;

/// A meridian is marked every this many degrees along the bottom edge.
const MERIDIAN_STEP: f64 = 15.0;
/// How finely the terminator is sampled, in degrees of longitude. Its curve is
/// gentle enough that this is well under a pixel of error at any window size.
const NIGHT_STEP: f64 = 2.0;

/// How far one wheel notch moves the shown instant.
const NOTCH_MINUTES: i64 = 15;
/// What counts as a notch when the wheel reports exact pixels, as a trackpad
/// does. Roughly the distance a mouse wheel's click carries.
const NOTCH_PIXELS: f32 = 40.0;

impl Elsewhere {
    /// Where the map last drew itself, in the coordinates of the region it
    /// fills. `None` until the first frame has been laid out.
    pub(super) fn map_rect(&self) -> Option<Bounds<Pixels>> {
        self.map_rect.get()
    }

    /// The map region: the world, the cities on it, and the wheel that scrubs
    /// the shown instant.
    pub(super) fn render_map(
        &self,
        saved: &[u32],
        now: &Zoned,
        dragging: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        div()
            .relative()
            .size_full()
            .on_scroll_wheel(
                cx.listener(|this, event: &ScrollWheelEvent, _window, cx| this.scrub(event, cx)),
            )
            .child(self.render_world(now))
            .children(self.render_pins(saved, now, dragging, window, cx))
    }

    /// Wheel over the map: a notch down is a quarter hour later, the way the
    /// night moves. The result is an ordinary pin, so the banner, the rows and
    /// the shade all follow it and escape still undoes it.
    fn scrub(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let notches = match event.delta {
            ScrollDelta::Lines(lines) => lines.y,
            ScrollDelta::Pixels(pixels) => f32::from(pixels.y) / NOTCH_PIXELS,
        };
        // A trackpad arrives as a stream of small deltas, so what does not add
        // up to a whole notch waits here for the next one.
        let carried = self.scrub_carry + notches;
        self.scrub_carry = carried.fract();
        let notches = carried.trunc();
        if notches == 0.0 {
            return;
        }

        // Without a zone to read the what-if time in there is nothing to pin.
        let Some(anchor) = self
            .pinned
            .as_ref()
            .map(|pin| pin.anchor)
            .or_else(|| self.first_resolved())
        else {
            return;
        };
        let from = match &self.pinned {
            Some(pin) => pin.at.clone(),
            None => quarter_hour(&Zoned::now()),
        };
        // Scrolling down pushes the content up, which the wheel reports as a
        // negative delta; that is the gesture that should move time forward.
        let Ok(at) = from.checked_add(Span::new().minutes(-(notches as i64) * NOTCH_MINUTES))
        else {
            return;
        };
        self.pinned = Some(Pin { anchor, at });
        cx.notify();
    }

    /// The first saved city whose zone this machine knows, which is what a
    /// scrub anchors to when no pin is carrying an anchor of its own.
    fn first_resolved(&self) -> Option<u32> {
        self.saved.iter().copied().find(|geonameid| {
            self.index
                .city(*geonameid)
                .is_some_and(|city| self.zones.get(&city.timezone).is_some())
        })
    }

    fn render_world(&self, now: &Zoned) -> impl IntoElement + use<> {
        let land = self.land.clone();
        let remembered = self.map_rect.clone();
        let sun = geo::subsolar(now);

        canvas(
            move |bounds, window, _cx| {
                let rect = fit(bounds.size);
                if remembered.get() != Some(rect) {
                    remembered.set(Some(rect));
                    // What goes over the map is positioned from this rect, so
                    // the frame that first measures it is already stale.
                    window.refresh();
                }
                Bounds {
                    origin: bounds.origin + rect.origin,
                    size: rect.size,
                }
            },
            move |_bounds, rect: Bounds<Pixels>, window, _cx| {
                let at = |place: LonLat| {
                    let (x, y) = geo::project(place);
                    point(
                        rect.origin.x + rect.size.width * x as f32,
                        rect.origin.y + rect.size.height * y as f32,
                    )
                };

                for ring in land.iter() {
                    if let Some(path) = polygon(ring.iter().copied().map(at)) {
                        window.paint_path(path, rgb(theme::SURFACE0));
                    }
                }

                if let Some(path) = polygon(geo::night(sun, NIGHT_STEP).into_iter().map(at)) {
                    window.paint_path(path, rgba(theme::NIGHT));
                }

                let mut lon = -180.0;
                while lon < 180.0 {
                    let x = at(LonLat { lon, lat: 0.0 }).x;
                    window.paint_quad(fill(
                        Bounds::new(
                            point(x - px(0.5), rect.bottom() - px(3.)),
                            size(px(1.), px(3.)),
                        ),
                        rgb(theme::SURFACE1),
                    ));
                    lon += MERIDIAN_STEP;
                }

                // The sun only ever stands between the tropics, but the crop is
                // an editable constant and clamping would park the mark on an
                // edge it never actually reaches.
                if (geo::SOUTH..=geo::NORTH).contains(&sun.lat) {
                    let radius = px(3.);
                    let centre = at(sun);
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                point(centre.x - radius, centre.y - radius),
                                size(radius * 2., radius * 2.),
                            ),
                            rgb(theme::YELLOW),
                        )
                        .corner_radii(radius),
                    );
                }
            },
        )
        .size_full()
    }
}

/// The letterbox: the largest rect of `geo::aspect()` that fits `area`, with an
/// origin relative to it. Full width when the width is what runs out first,
/// otherwise centred across it; top-aligned either way, so the map stays put as
/// the window grows taller.
pub(super) fn fit(area: Size<Pixels>) -> Bounds<Pixels> {
    let aspect = geo::aspect() as f32;
    let height = area.width / aspect;
    if height <= area.height {
        Bounds::new(point(px(0.), px(0.)), size(area.width, height))
    } else {
        let width = area.height * aspect;
        Bounds::new(
            point((area.width - width) / 2., px(0.)),
            size(width, area.height),
        )
    }
}

/// `at` with its seconds and any odd minutes dropped, so a scrub that starts
/// from the real clock lands on a round quarter rather than carrying the
/// current second along forever.
fn quarter_hour(at: &Zoned) -> Zoned {
    at.round(
        ZonedRound::new()
            .smallest(Unit::Minute)
            .increment(NOTCH_MINUTES)
            .mode(RoundMode::Trunc),
    )
    .expect("a quarter hour is a valid rounding increment")
}

/// A closed filled path through `points`, or `None` for a run too short to
/// enclose anything or one lyon cannot tessellate.
fn polygon(points: impl Iterator<Item = Point<Pixels>>) -> Option<Path<Pixels>> {
    let points: Vec<_> = points.collect();
    if points.len() < 3 {
        return None;
    }
    let mut builder = PathBuilder::fill();
    builder.add_polygon(&points, true);
    builder.build().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: Pixels, want: f32) -> bool {
        (f32::from(got) - want).abs() < 0.01
    }

    #[test]
    fn fills_the_width_when_the_area_is_the_taller_shape() {
        let rect = fit(size(px(400.), px(300.)));
        assert_eq!(rect.origin, point(px(0.), px(0.)));
        assert_eq!(rect.size.width, px(400.));
        assert!(
            close(rect.size.height, 400. / (360. / 140.)),
            "{rect:?} is not the map's aspect"
        );
    }

    #[test]
    fn centres_across_the_width_when_the_height_is_the_limit() {
        let rect = fit(size(px(400.), px(100.)));
        assert_eq!(rect.origin.y, px(0.));
        assert_eq!(rect.size.height, px(100.));
        assert!(
            close(rect.size.width, 100. * (360. / 140.)),
            "{rect:?} is not the map's aspect"
        );
        assert!(
            close(rect.origin.x + rect.size.width / 2., 200.),
            "{rect:?} is not centred"
        );
    }
}
