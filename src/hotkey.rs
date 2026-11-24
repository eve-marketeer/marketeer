//! The system-wide shortcut that switches between selling and buying without leaving the
//! game.
//!
//! Registered through `global-hotkey`: works on Windows, macOS and X11. On a Wayland session
//! only X11 windows (EVE under Proton is one) can trigger it.

use std::str::FromStr;

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

/// Cmd+M on a Mac, Ctrl+M elsewhere.
pub(crate) const DEFAULT: &str = if cfg!(target_os = "macos") {
    "super+KeyM"
} else {
    "control+KeyM"
};

/// A key press in the app's window as a shortcut, or `None` when it cannot be one: a bare
/// modifier, or a key without Control, Alt or the platform key (F-keys excepted).
pub(crate) fn from_keystroke(keystroke: &gpui_kit::Keystroke) -> Option<String> {
    let m = &keystroke.modifiers;
    let key = keystroke.key.to_lowercase();
    let code = match key.as_str() {
        k if k.len() == 1 && k.chars().all(|c| c.is_ascii_alphabetic()) => {
            format!("Key{}", k.to_uppercase())
        }
        k if k.len() == 1 && k.chars().all(|c| c.is_ascii_digit()) => format!("Digit{k}"),
        k if k.starts_with('f') && k[1..].parse::<u8>().is_ok_and(|n| (1..=24).contains(&n)) => {
            k.to_uppercase()
        }
        "space" => "Space".into(),
        "up" => "ArrowUp".into(),
        "down" => "ArrowDown".into(),
        "left" => "ArrowLeft".into(),
        "right" => "ArrowRight".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "pageup" => "PageUp".into(),
        "pagedown" => "PageDown".into(),
        "insert" => "Insert".into(),
        "-" => "Minus".into(),
        "=" => "Equal".into(),
        "[" => "BracketLeft".into(),
        "]" => "BracketRight".into(),
        ";" => "Semicolon".into(),
        "'" => "Quote".into(),
        "," => "Comma".into(),
        "." => "Period".into(),
        "/" => "Slash".into(),
        "\\" => "Backslash".into(),
        "`" => "Backquote".into(),
        _ => return None,
    };
    let function_key = code.starts_with('F') && code.len() <= 3;
    if !(m.control || m.alt || m.platform || function_key) {
        return None;
    }
    let mut parts = Vec::new();
    if m.control {
        parts.push("control".to_string());
    }
    if m.alt {
        parts.push("alt".into());
    }
    if m.shift {
        parts.push("shift".into());
    }
    if m.platform {
        parts.push("super".into());
    }
    parts.push(code);
    let text = parts.join("+");
    HotKey::from_str(&text).ok().map(|_| text)
}

/// `control+alt+KeyR` as its keys, the way people write them: `Ctrl`, `Alt`, `R`.
pub(crate) fn keys(hotkey: &str) -> Vec<String> {
    hotkey
        .split('+')
        .filter(|part| !part.is_empty())
        .map(|part| match part.to_lowercase().as_str() {
            "control" | "ctrl" => "Ctrl".to_string(),
            "alt" | "option" => if cfg!(target_os = "macos") {
                "Option"
            } else {
                "Alt"
            }
            .into(),
            "shift" => "Shift".into(),
            "super" | "meta" | "command" | "cmd" => if cfg!(target_os = "macos") {
                "Cmd"
            } else {
                "Super"
            }
            .into(),
            _ => part
                .strip_prefix("Key")
                .or_else(|| part.strip_prefix("Digit"))
                .or_else(|| part.strip_prefix("Arrow"))
                .unwrap_or(part)
                .to_string(),
        })
        .collect()
}

/// The shortcut registered with the system right now.
pub(crate) struct Registry {
    manager: Option<GlobalHotKeyManager>,
    bound: Option<HotKey>,
    /// Why the shortcut is unavailable, when it is.
    pub problem: Option<String>,
}

impl Registry {
    /// Must run on the main thread, which is where GPUI calls the app from.
    pub(crate) fn new() -> Registry {
        match GlobalHotKeyManager::new() {
            Ok(manager) => Registry {
                manager: Some(manager),
                bound: None,
                problem: None,
            },
            Err(err) => Registry {
                manager: None,
                bound: None,
                problem: Some(format!("Global shortcuts are unavailable here: {err}")),
            },
        }
    }

    /// Replace what is registered with `hotkey`; empty registers nothing. Returns why the
    /// system refused, usually because another program holds the keys.
    pub(crate) fn bind(&mut self, hotkey: &str) -> Option<String> {
        let manager = self.manager.as_ref()?;
        if let Some(old) = self.bound.take() {
            let _ = manager.unregister(old);
        }
        if hotkey.is_empty() {
            return None;
        }
        let Ok(parsed) = HotKey::from_str(hotkey) else {
            return Some(format!("{hotkey} is not a shortcut"));
        };
        match manager.register(parsed) {
            Ok(()) => {
                self.bound = Some(parsed);
                None
            }
            Err(err) => Some(format!("{}: {err}", keys(hotkey).join("+"))),
        }
    }

    /// Whether the shortcut was pressed since the last call.
    pub(crate) fn pressed(&self) -> bool {
        let mut pressed = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state() == HotKeyState::Pressed
                && self.bound.is_some_and(|bound| bound.id() == event.id())
            {
                pressed = true;
            }
        }
        pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_parses_and_reads_well() {
        assert!(HotKey::from_str(DEFAULT).is_ok());
        assert_eq!(keys("control+shift+KeyM"), ["Ctrl", "Shift", "M"]);
        assert_eq!(
            keys("alt+F5"),
            [
                if cfg!(target_os = "macos") {
                    "Option"
                } else {
                    "Alt"
                },
                "F5"
            ]
        );
        assert_eq!(keys(""), Vec::<String>::new());
    }
}
