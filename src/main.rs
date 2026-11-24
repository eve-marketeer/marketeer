// No console window behind the app on Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod bulk;
mod config;
mod esi;
mod hotkey;
mod market;
mod theme;
mod ui;
mod update;
mod watcher;

use gpui_kit::{App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

/// A line on stderr, the app's only log.
#[expect(clippy::print_stderr, reason = "stderr is the desktop app's only log")]
pub(crate) fn log(line: std::fmt::Arguments<'_>) {
    eprintln!("{line}");
}

fn main() {
    let application = gpui_kit::application().with_assets(theme::Assets);
    application.run(|cx| {
        gpui_kit::init(cx);
        theme::init(cx);
        open_main(cx);
        // One window: closing it is quitting.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
    });
}

fn open_main(cx: &mut App) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(820.), px(760.)),
            cx,
        ))),
        window_min_size: Some(size(px(640.), px(520.))),
        titlebar: Some(TitlebarOptions {
            title: Some("Marketeer".into()),
            ..Default::default()
        }),
        app_id: Some("marketeer".into()),
        ..Default::default()
    };
    match gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| app::Marketeer::new(window, cx))
    }) {
        Ok(_) => cx.activate(true),
        Err(err) => log(format_args!("could not open the window: {err:#}")),
    }
}
