//! The palette, Catppuccin Mocha. `rgb()` is not `const` in gpui, so these stay
//! plain `u32` and call sites wrap them: `rgb(theme::BASE)`.

/// The window's background.
pub(crate) const BASE: u32 = 0x1e1e2e;
/// Panels raised off the window: the results overlay, the drag preview.
pub(crate) const MANTLE: u32 = 0x181825;
/// The hover wash under anything clickable.
pub(crate) const SURFACE0: u32 = 0x313244;
/// The same wash at half strength, under a hovered row. Carries its alpha, so
/// call sites wrap it in `rgba()`.
pub(crate) const WASH: u32 = 0x31324480;
/// The scrollbar thumb.
pub(crate) const SURFACE1: u32 = 0x45475a;
/// Text that steps back: the day tag, a toggle that is off.
pub(crate) const OVERLAY0: u32 = 0x6c7086;
/// Text that reads as secondary: row labels, the grip, the drag preview.
pub(crate) const SUBTEXT0: u32 = 0xa6adc8;
/// The default text color.
pub(crate) const TEXT: u32 = 0xcdd6f4;
/// Destructive: the bin, with a row over it.
pub(crate) const RED: u32 = 0xf38ba8;
/// A what-if time is in effect: pinned readings, the banner.
pub(crate) const YELLOW: u32 = 0xf9e2af;
/// The banner's fill and edge: the same yellow, faint and less faint. Both
/// carry their alpha, so call sites wrap them in `rgba()`.
pub(crate) const YELLOW_TINT: u32 = 0xf9e2af14;
pub(crate) const YELLOW_EDGE: u32 = 0xf9e2af40;
/// Active: the westward toggle on, the drag insertion line.
pub(crate) const BLUE: u32 = 0x89b4fa;
