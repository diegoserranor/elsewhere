//! The palette, Catppuccin Mocha. `rgb()` is not `const` in gpui, so these stay
//! plain `u32` and call sites wrap them: `rgb(theme::BASE)`.

/// The window's background.
pub(crate) const BASE: u32 = 0x1e1e2e;
/// Panels raised off the window: the results overlay, the drag preview. Also
/// the trough the view switch sits in.
pub(crate) const MANTLE: u32 = 0x181825;
/// The hover wash under anything clickable.
pub(crate) const SURFACE0: u32 = 0x313244;
/// The same wash at half strength, under a hovered row. Carries its alpha, so
/// call sites wrap it in `rgba()`.
pub(crate) const WASH: u32 = 0x31324480;
/// The map's dark half, laid over the land. Catppuccin crust, and it carries
/// its alpha, so call sites wrap it in `rgba()`.
pub(crate) const NIGHT: u32 = 0x11111b99;
/// The time zone borders on the map. Opaque, so the edge two zones share
/// and both draw reads as one line.
pub(crate) const ZONE_LINE: u32 = 0x45475a;
/// The scrollbar thumb.
pub(crate) const SURFACE1: u32 = 0x45475a;
/// Text that steps back: the day tag, a toggle that is off.
pub(crate) const OVERLAY0: u32 = 0x6c7086;
/// Text that reads as secondary: row labels, the grip, the drag preview.
pub(crate) const SUBTEXT0: u32 = 0xa6adc8;
/// The default text color.
pub(crate) const TEXT: u32 = 0xcdd6f4;
/// A what-if time typed into a row is in effect: pinned readings, the banner.
/// Also the sun on the map.
pub(crate) const YELLOW: u32 = 0xf9e2af;
/// The banner's fill and edge: the same yellow, faint and less faint. Both
/// carry their alpha, so call sites wrap them in `rgba()`.
pub(crate) const YELLOW_TINT: u32 = 0xf9e2af14;
pub(crate) const YELLOW_EDGE: u32 = 0xf9e2af40;
/// A what-if time scrubbed to with no row behind it, as the wheel over the
/// map gives: the yellow's three roles, in mauve.
pub(crate) const MAUVE: u32 = 0xcba6f7;
pub(crate) const MAUVE_TINT: u32 = 0xcba6f714;
pub(crate) const MAUVE_EDGE: u32 = 0xcba6f740;
/// Active: the view shown, the westward toggle on, the drag insertion line, the bin with a
/// row over it.
pub(crate) const BLUE: u32 = 0x89b4fa;
