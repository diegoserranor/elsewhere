//! The heads over the grid's columns: the city's name, which is the handle
//! it is dragged by as the map's dot is, over the offset it stands at. Set
//! flush right over the labels, as a calendar sets its zone names.

use gpui::{Context, Pixels, Styled, div, prelude::*, px, rgb};
use jiff::Zoned;

use super::{COLUMN, Column, INSET, LABEL};
use crate::app::Elsewhere;
use crate::app::drag::DragRow;
use crate::app::tooltip::Tooltip;
use crate::clock;
use crate::theme;

/// The strip's height: two label lines and the air under them. Fixed, so a
/// column with no offset line keeps the rules level, and the hours' scrollbar
/// knows where to start.
pub(super) const HEAD: Pixels = px(40.);

impl Elsewhere {
    /// The strip of heads, one per column.
    pub(super) fn render_heads(
        &self,
        columns: &[Column],
        now: &Zoned,
        dragging: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        div().flex_none().h(HEAD).flex().flex_row().children(
            columns
                .iter()
                .map(|column| self.render_head(column, now, dragging, cx)),
        )
    }

    /// One column: its name, then its offset now. The name is all the room
    /// there is, so the tooltip carries the full label and what the zone is
    /// called today.
    fn render_head(
        &self,
        column: &Column,
        now: &Zoned,
        dragging: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let offset = column.zone.as_ref().map(|zone| {
            shortened(div())
                .text_color(rgb(theme::OVERLAY0))
                .child(clock::offset_label(zone.to_offset(now.timestamp())))
        });
        let head = div()
            .flex_1()
            .min_w(COLUMN)
            .pr(INSET)
            .flex()
            .flex_col()
            .justify_end()
            .pb_2();
        let Some(geonameid) = column.geonameid else {
            let tip = column
                .zone
                .as_ref()
                .map(|zone| clock::zone_detail(zone, now))
                .unwrap_or_default();
            return head
                .child(
                    shortened(div().id("head-here"))
                        .text_color(rgb(theme::OVERLAY0))
                        .tooltip(Tooltip::text(tip))
                        .child("here"),
                )
                .children(offset);
        };
        let Some(city) = self.index.city(geonameid) else {
            return head;
        };
        let label = self.index.label(city);
        let tip = match &column.zone {
            Some(zone) => format!("{label} · {}", clock::zone_detail(zone, now)),
            None => label.clone(),
        };
        head.child(
            shortened(div().id(("head", geonameid as usize)))
                .text_color(rgb(theme::SUBTEXT0))
                .tooltip(Tooltip::text(tip))
                .cursor_grab()
                .on_drag(DragRow { geonameid, label }, Self::drag_preview(cx))
                .when(dragging && self.drag == Some(geonameid), |name| {
                    name.opacity(0.4)
                })
                .child(city.name.clone()),
        )
        .children(offset)
    }
}

/// A head's line, ended in an ellipsis when the column is too narrow to hold
/// it. This is `truncate` written out with the text left wrappable, which is
/// what puts the ellipsis in the right place: nowrap text has no wrap width,
/// and gpui keeps the first shaping of such a line for the rest of the frame —
/// one of the passes taffy takes to size the column, against a width of zero
/// or of the whole name, and never the width the head ends up with.
fn shortened<T: Styled>(line: T) -> T {
    line.text_ellipsis()
        .line_clamp(1)
        .text_size(LABEL)
        .text_right()
}
