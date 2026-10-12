//! The icons the window draws, baked into the binary. gpui looks an `svg()`
//! up by path through the application's `AssetSource`; this one knows the
//! handful under `assets/` and nothing else.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(match path {
            "icons/trash.svg" => Some(Cow::Borrowed(include_bytes!("../assets/icons/trash.svg"))),
            "icons/milestone.svg" => Some(Cow::Borrowed(include_bytes!(
                "../assets/icons/milestone.svg"
            ))),
            "icons/clock-12.svg" => Some(Cow::Borrowed(include_bytes!(
                "../assets/icons/clock-12.svg"
            ))),
            "icons/map.svg" => Some(Cow::Borrowed(include_bytes!("../assets/icons/map.svg"))),
            "icons/rows.svg" => Some(Cow::Borrowed(include_bytes!("../assets/icons/rows.svg"))),
            "icons/columns.svg" => {
                Some(Cow::Borrowed(include_bytes!("../assets/icons/columns.svg")))
            }
            _ => None,
        })
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
