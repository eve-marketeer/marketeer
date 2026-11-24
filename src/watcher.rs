//! Watches the export folder, so a new export shows up the moment the game writes it.

use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::market;

/// How long the folder has to be quiet before the export is read: the game creates the
/// file, then writes it.
const SETTLE: Duration = Duration::from_millis(150);

pub(crate) struct Watcher {
    events: Receiver<()>,
    /// When the folder last changed, while that change has not been read yet.
    changed: Option<Instant>,
    _notify: RecommendedWatcher,
}

impl Watcher {
    pub(crate) fn new(dir: &Path) -> Result<Watcher, String> {
        let (tx, events) = channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let Ok(event) = event
                    && matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_))
                    && event.paths.iter().any(|path| market::is_export(path))
                {
                    let _ = tx.send(());
                }
            })
            .map_err(|e| e.to_string())?;
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(|e| e.to_string())?;
        Ok(Watcher {
            events,
            changed: None,
            _notify: watcher,
        })
    }

    /// Whether an export was written and the folder has been quiet since. Called often.
    pub(crate) fn settled(&mut self) -> bool {
        if self.events.try_iter().count() > 0 {
            self.changed = Some(Instant::now());
        }
        if self.changed.is_some_and(|at| at.elapsed() >= SETTLE) {
            self.changed = None;
            return true;
        }
        false
    }
}
