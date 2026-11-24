//! The look: near-black with green for selling and orange for buying, as the app has
//! always had.

use std::borrow::Cow;
use std::rc::Rc;

use gpui_kit::component::{Theme, ThemeMode, ThemeSet};
use gpui_kit::{App, AssetSource, Hsla, SharedString, rgb};

use crate::market::Mode;

const THEMES: &str = include_str!("../assets/theme.json");

/// Inter, bundled so the app looks the same everywhere (OFL, license next to the files).
const FONTS: [&[u8]; 4] = [
    include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../assets/fonts/Inter-Medium.ttf"),
    include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../assets/fonts/Inter-Bold.ttf"),
];

/// The kit's Lucide icons, plus the Marketeer mark at `logo.svg`.
pub(crate) struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<Cow<'static, [u8]>>> {
        if path == "logo.svg" {
            return Ok(Some(Cow::Borrowed(include_bytes!("../assets/logo.svg"))));
        }
        gpui_kit::assets::AllAssets.load(path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<SharedString>> {
        gpui_kit::assets::AllAssets.list(path)
    }
}

/// Load the theme. Once, before the first window. The app is dark whatever the system is.
pub(crate) fn init(cx: &mut App) {
    if let Err(err) = cx
        .text_system()
        .add_fonts(FONTS.iter().map(|f| Cow::Borrowed(*f)).collect())
    {
        crate::log(format_args!("could not load the fonts: {err}"));
    }
    let set: ThemeSet = serde_json::from_str(THEMES).expect("assets/theme.json is valid");
    Theme::change(ThemeMode::Dark, None, cx);
    let theme = Theme::global_mut(cx);
    for config in set.themes {
        theme.apply_config(&Rc::new(config));
    }
}

fn hex(value: u32) -> Hsla {
    rgb(value).into()
}

pub(crate) fn sell() -> Hsla {
    hex(0x30bd44)
}

pub(crate) fn buy() -> Hsla {
    hex(0xff8918)
}

/// The color of `mode`'s side of the market.
pub(crate) fn side(mode: Mode) -> Hsla {
    match mode {
        Mode::Sell => sell(),
        Mode::Buy => buy(),
    }
}

/// A hairline between things.
pub(crate) fn hairline() -> Hsla {
    hex(0xffffff).opacity(0.07)
}
