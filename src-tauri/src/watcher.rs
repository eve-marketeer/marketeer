use crate::market::{get_market_logs_path, parse_latest_log, MARKET_STATE};
use clipboard_rs::{Clipboard, ClipboardContext};
use notify::{Event, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub fn start_watching(app: AppHandle) {
    let Some(logs_path) = get_market_logs_path() else {
        eprintln!("Could not find EVE market logs directory");
        return;
    };
    
    if !logs_path.exists() {
        eprintln!("EVE market logs directory does not exist: {:?}", logs_path);
        return;
    }
    
    let (tx, rx) = channel();
    
    let mut watcher = match notify::recommended_watcher(move |res: Result<Event, _>| {
        if let Ok(event) = res {
            if event.kind.is_create() || event.kind.is_modify() {
                let _ = tx.send(());
            }
        }
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to create file watcher: {}", e);
            return;
        }
    };
    
    if let Err(e) = watcher.watch(&logs_path, RecursiveMode::NonRecursive) {
        eprintln!("Failed to watch directory: {}", e);
        return;
    }
    
    println!("Watching for market logs in: {:?}", logs_path);
    
    // Debounce timer
    let mut last_event = std::time::Instant::now();
    let debounce_duration = Duration::from_millis(500);
    
    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(()) => {
                let now = std::time::Instant::now();
                if now.duration_since(last_event) < debounce_duration {
                    continue;
                }
                last_event = now;
                
                // Small delay to ensure file is fully written
                std::thread::sleep(Duration::from_millis(100));
                
                let data = parse_latest_log();
                
                // Update state and copy price
                {
                    let mut state = MARKET_STATE.lock();
                    state.data = data.clone();
                    
                    if let Some(price) = state.get_active_price() {
                        if let Ok(ctx) = ClipboardContext::new() {
                            let _ = ctx.set_text(format!("{:.2}", price));
                        }
                    }
                }
                
                // Emit event to frontend
                let _ = app.emit("market-updated", &data);
                println!("Market data updated: {:?}", data.item_name);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Just keep watching
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("File watcher channel disconnected");
                break;
            }
        }
    }
}

