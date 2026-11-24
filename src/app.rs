//! The window: Market shows the newest export and the price that beats it, Bulk prices a
//! pasted list against Jita, and Settings holds the folder, the shortcut and updates. A new
//! export or the shortcut puts the price on the clipboard.

mod pages;

use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::{
    AppContext, Context, Entity, FocusHandle, Image, ImageFormat, KeyDownEvent, Subscription, Task,
    Window,
};

use crate::bulk::{self, Row};
use crate::config::Config;
use crate::esi::{self, Day, TypeInfo};
use crate::hotkey::{self, Registry};
use crate::market::{self, Export, Mode};
use crate::ui::ChartBounds;
use crate::update::{self, Available};
use crate::watcher::Watcher;

/// How often the shortcut and the export folder are looked at.
const TICK: Duration = Duration::from_millis(50);
/// How long "Copied" stays up.
const COPIED_FOR: Duration = Duration::from_secs(2);
/// How long an item's name, icon and history are kept before asking again.
const DETAILS_FOR: Duration = Duration::from_hours(1);
const UPDATE_EVERY: Duration = Duration::from_hours(6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Market,
    Bulk,
    Settings,
}

impl Page {
    const ALL: [Page; 3] = [Page::Market, Page::Bulk, Page::Settings];

    fn label(self) -> &'static str {
        match self {
            Page::Market => "Market",
            Page::Bulk => "Bulk undercut",
            Page::Settings => "Settings",
        }
    }
}

/// What ESI knows about the exported item.
#[derive(Clone)]
struct Details {
    type_id: i64,
    info: Option<TypeInfo>,
    icon: Option<Arc<Image>>,
    history: Rc<[Day]>,
}

/// The bulk tool's state.
#[derive(Default)]
struct Bulk {
    rows: Vec<Row>,
    loading: bool,
    problem: Option<String>,
    /// When the rows were last copied.
    copied: Option<Instant>,
    /// Tells one run from the next, so a slow one does not overwrite a newer one.
    run: u64,
}

/// What the updater is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Updater {
    Idle,
    Checking,
    Installing,
}

pub(crate) struct Marketeer {
    config: Config,
    page: Page,
    /// The export folder in use: the one from Settings, else the one found.
    logs_dir: Option<PathBuf>,
    watcher: Option<Watcher>,
    export: Option<Export>,
    details: Option<Details>,
    details_cache: HashMap<i64, (Instant, Details)>,
    /// The price put on the clipboard last, and when.
    copied: Option<(f64, Instant)>,
    hotkeys: Registry,
    /// Waiting for the keys of a new shortcut.
    capturing: bool,
    focus: FocusHandle,
    clipboard: Option<arboard::Clipboard>,
    /// What went wrong, until dismissed.
    problems: Vec<String>,
    logs_input: Entity<InputState>,
    bulk_input: Entity<TextareaState>,
    bulk: Bulk,
    chart_bounds: ChartBounds,
    chart_hover: Option<usize>,
    update: Option<Available>,
    updater: Updater,
    update_note: Option<String>,
    tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Marketeer {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let (config, config_problem) = Config::load();
        let logs_dir = config.logs_dir.clone().or_else(market::default_dir);
        let logs_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Documents/EVE/logs/Marketlogs")
                .default_value(
                    logs_dir
                        .as_ref()
                        .map_or_default(|p| p.display().to_string()),
                )
        });
        let bulk_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Multispectrum Shield Hardener II    7\n1600mm Steel Plates II    2")
        });
        let subscriptions = vec![
            cx.subscribe_in(&logs_input, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    this.use_logs_dir(cx);
                }
            }),
            // Ctrl+Enter in the list runs it.
            cx.subscribe_in(&bulk_input, window, |this, _, event: &InputEvent, _, cx| {
                if matches!(
                    event,
                    InputEvent::PressEnter {
                        secondary: true,
                        ..
                    }
                ) {
                    this.run_bulk(cx);
                }
            }),
        ];

        let mut hotkeys = Registry::new();
        let refused = hotkeys.bind(&config.hotkey);
        let mut problems: Vec<String> = [config_problem, hotkeys.problem.clone(), refused]
            .into_iter()
            .flatten()
            .collect();
        let clipboard = arboard::Clipboard::new().ok();
        if clipboard.is_none() {
            problems.push("The clipboard is unavailable, so prices cannot be copied".into());
        }

        let mut this = Marketeer {
            config,
            page: Page::Market,
            logs_dir: None,
            watcher: None,
            export: None,
            details: None,
            details_cache: HashMap::new(),
            copied: None,
            hotkeys,
            capturing: false,
            focus: cx.focus_handle(),
            clipboard,
            problems,
            logs_input,
            bulk_input,
            bulk: Bulk::default(),
            chart_bounds: Rc::new(Cell::new(None)),
            chart_hover: None,
            update: None,
            updater: Updater::Idle,
            update_note: None,
            tasks: Vec::new(),
            _subscriptions: subscriptions,
        };
        // So the page keys work before anything is clicked.
        window.focus(&this.focus, cx);
        this.watch(logs_dir);
        // What was exported before the app started is shown, not copied: the clipboard
        // is only taken for something that just happened.
        this.reload(cx);
        this.tasks.push(Self::tick_loop(cx));
        this.tasks.push(Self::update_loop(cx));
        this
    }

    // The export folder.

    fn watch(&mut self, dir: Option<PathBuf>) {
        self.watcher = None;
        self.logs_dir = dir.filter(|d| d.is_dir());
        let Some(dir) = &self.logs_dir else { return };
        match Watcher::new(dir) {
            Ok(watcher) => self.watcher = Some(watcher),
            Err(err) => self.problems.push(format!(
                "Could not watch {}: {err}. Use Refresh after exporting.",
                dir.display()
            )),
        }
    }

    /// Take the folder typed in Settings. Empty goes back to the one found.
    fn use_logs_dir(&mut self, cx: &mut Context<'_, Self>) {
        let typed = self.logs_input.read(cx).value().trim().to_string();
        let chosen = (!typed.is_empty()).then(|| PathBuf::from(typed));
        let found = market::default_dir();
        // The folder found on its own is not a choice worth saving.
        let saved = chosen.clone().filter(|dir| Some(dir) != found.as_ref());
        let wanted = chosen.or(found);
        if saved == self.config.logs_dir && wanted.clone().filter(|d| d.is_dir()) == self.logs_dir {
            return;
        }
        self.config.logs_dir = saved;
        self.save();
        self.watch(wanted);
        self.reload(cx);
    }

    /// Read the newest export again.
    fn reload(&mut self, cx: &mut Context<'_, Self>) {
        self.export = self.logs_dir.as_deref().and_then(market::read_latest);
        self.chart_hover = None;
        self.load_details(cx);
        cx.notify();
    }

    /// The Refresh button: read again, and copy like a new export would.
    fn refresh(&mut self, cx: &mut Context<'_, Self>) {
        self.reload(cx);
        self.copy_price(cx);
    }

    // The price.

    fn price_now(&self) -> Option<f64> {
        self.export.as_ref()?.price(self.config.mode)
    }

    /// Put the price for the current mode on the clipboard.
    fn copy_price(&mut self, cx: &mut Context<'_, Self>) {
        let Some(price) = self.price_now() else {
            return;
        };
        if !self.copy(format!("{price:.2}")) {
            return;
        }
        self.copied = Some((price, Instant::now()));
        cx.notify();
        Self::notify_after(COPIED_FOR, cx);
    }

    fn copy(&mut self, text: String) -> bool {
        self.clipboard
            .as_mut()
            .is_some_and(|clipboard| clipboard.set_text(text).is_ok())
    }

    /// Draw again once something timed has run out.
    fn notify_after(delay: Duration, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update(cx, |_, cx| cx.notify()).ok();
        })
        .detach();
    }

    fn recently_copied(&self) -> bool {
        self.copied.is_some_and(|(_, at)| at.elapsed() < COPIED_FOR)
    }

    fn set_mode(&mut self, mode: Mode, cx: &mut Context<'_, Self>) {
        if self.config.mode != mode {
            self.config.mode = mode;
            self.save();
        }
        self.copy_price(cx);
        cx.notify();
    }

    fn toggle_mode(&mut self, cx: &mut Context<'_, Self>) {
        self.set_mode(self.config.mode.other(), cx);
    }

    fn save(&mut self) {
        if let Err(err) = self.config.save() {
            self.problems
                .push(format!("Could not save the settings: {err}"));
        }
    }

    /// The shortcut and the folder, looked at a few times a second.
    fn tick_loop(cx: &mut Context<'_, Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let alive = this.update(cx, |this, cx| {
                    if this.hotkeys.pressed() {
                        this.toggle_mode(cx);
                    }
                    if this.watcher.as_mut().is_some_and(Watcher::settled) {
                        this.reload(cx);
                        this.copy_price(cx);
                    }
                });
                if alive.is_err() {
                    return;
                }
            }
        })
    }

    // The item.

    /// Get the exported item's name, icon and history, unless they are here already.
    fn load_details(&mut self, cx: &mut Context<'_, Self>) {
        let Some(type_id) = self.export.as_ref().and_then(|e| e.type_id) else {
            self.details = None;
            return;
        };
        if let Some((at, details)) = self.details_cache.get(&type_id)
            && at.elapsed() < DETAILS_FOR
        {
            self.details = Some(details.clone());
            return;
        }
        if self.details.as_ref().is_some_and(|d| d.type_id != type_id) {
            self.details = None;
        }
        let fetch = cx.background_spawn(async move {
            (
                esi::type_info(type_id),
                esi::icon(type_id),
                esi::history(type_id),
            )
        });
        cx.spawn(async move |this, cx| {
            let (info, icon, history) = fetch.await;
            this.update(cx, |this, cx| {
                // Nothing came back, as when offline: ask again on the next export.
                let complete = info.is_some() && !history.is_empty();
                let details = Details {
                    type_id,
                    info,
                    icon: icon.and_then(image),
                    history: history.into(),
                };
                if complete {
                    this.details_cache
                        .insert(type_id, (Instant::now(), details.clone()));
                }
                if this.export.as_ref().and_then(|e| e.type_id) == Some(type_id) {
                    this.details = Some(details);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    // The bulk tool.

    fn run_bulk(&mut self, cx: &mut Context<'_, Self>) {
        let text = self.bulk_input.read(cx).value().to_string();
        if text.trim().is_empty() || self.bulk.loading {
            return;
        }
        self.bulk.run += 1;
        let run = self.bulk.run;
        self.bulk.loading = true;
        self.bulk.problem = None;
        self.bulk.rows.clear();
        cx.notify();
        let work = cx.background_spawn(async move { bulk::run(&text) });
        cx.spawn(async move |this, cx| {
            let done = work.await;
            this.update(cx, |this, cx| {
                if this.bulk.run != run {
                    return;
                }
                this.bulk.loading = false;
                match done {
                    Ok(rows) => this.bulk.rows = rows,
                    Err(err) => this.bulk.problem = Some(err),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn copy_bulk(&mut self, cx: &mut Context<'_, Self>) {
        let text = bulk::for_clipboard(&self.bulk.rows);
        if text.is_empty() || !self.copy(text) {
            return;
        }
        self.bulk.copied = Some(Instant::now());
        cx.notify();
        Self::notify_after(COPIED_FOR, cx);
    }

    // The shortcut.

    fn start_capture(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        self.capturing = true;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn captured(&mut self, event: &KeyDownEvent, cx: &mut Context<'_, Self>) {
        if !self.capturing {
            return;
        }
        let key = event.keystroke.key.as_str();
        let chosen = if key == "escape" {
            None
        } else if key == "backspace" || key == "delete" {
            Some(String::new())
        } else if let Some(hotkey) = hotkey::from_keystroke(&event.keystroke) {
            Some(hotkey)
        } else {
            // A modifier on its way down, or a key that cannot be a shortcut.
            return;
        };
        self.capturing = false;
        if let Some(hotkey) = chosen {
            self.rebind(hotkey);
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn rebind(&mut self, hotkey: String) {
        if let Some(why) = self.hotkeys.bind(&hotkey) {
            self.problems.push(why);
        }
        self.config.hotkey = hotkey;
        self.save();
    }

    /// Ctrl or Cmd with 1, 2 or 3 opens that page.
    fn page_key(&mut self, event: &KeyDownEvent, cx: &mut Context<'_, Self>) {
        if !event.keystroke.modifiers.secondary(cx) {
            return;
        }
        let page = match event.keystroke.key.as_str() {
            "1" => Page::Market,
            "2" => Page::Bulk,
            "3" => Page::Settings,
            _ => return,
        };
        self.page = page;
        cx.stop_propagation();
        cx.notify();
    }

    // Updates.

    /// Look for a new release now, and again every few hours.
    fn update_loop(cx: &mut Context<'_, Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                if this.update(cx, Marketeer::check_for_update).is_err() {
                    return;
                }
                cx.background_executor().timer(UPDATE_EVERY).await;
            }
        })
    }

    fn check_for_update(&mut self, cx: &mut Context<'_, Self>) {
        if self.updater != Updater::Idle {
            return;
        }
        self.updater = Updater::Checking;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let found = cx.background_spawn(async { update::check() }).await;
            this.update(cx, |this, cx| {
                // An install started meanwhile is still going.
                if this.updater == Updater::Checking {
                    this.updater = Updater::Idle;
                }
                match found {
                    Ok(Some(available)) => {
                        this.update = Some(available);
                        this.update_note = None;
                    }
                    Ok(None) => {
                        this.update = None;
                        this.update_note = Some("This is the newest version.".into());
                    }
                    Err(err) => this.update_note = Some(err),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Install in place where this copy can be, else open the release page.
    fn install_update(&mut self, cx: &mut Context<'_, Self>) {
        let Some(available) = self.update.clone() else {
            return;
        };
        let Some(installer) = available.installer else {
            cx.open_url(&available.page);
            return;
        };
        if self.updater == Updater::Installing {
            return;
        }
        self.updater = Updater::Installing;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_spawn(async move { update::install(&installer) })
                .await;
            this.update(cx, |this, cx| match done {
                Ok(()) => update::restart(cx),
                Err(err) => {
                    this.updater = Updater::Idle;
                    this.problems.push(format!("Could not update: {err}"));
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}

/// The image server's bytes as an image. It sends PNG for icons, JPEG for some renders.
fn image(bytes: Vec<u8>) -> Option<Arc<Image>> {
    let format = match bytes.as_slice() {
        [0x89, b'P', b'N', b'G', ..] => ImageFormat::Png,
        [0xff, 0xd8, ..] => ImageFormat::Jpeg,
        _ => return None,
    };
    Some(Arc::new(Image::from_bytes(format, bytes)))
}
