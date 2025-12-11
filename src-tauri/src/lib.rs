mod market;
mod esi;
mod watcher;

use clipboard_rs::{Clipboard, ClipboardContext};
use market::{MarketData, MarketMode, MARKET_STATE};
#[cfg(not(target_os = "macos"))]
use tauri::Manager;
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn get_market_data() -> MarketData {
    MARKET_STATE.lock().data.clone()
}

#[tauri::command]
fn get_current_mode() -> MarketMode {
    MARKET_STATE.lock().mode.clone()
}

#[tauri::command]
fn toggle_mode(app: AppHandle) -> MarketMode {
    let mut state = MARKET_STATE.lock();
    state.mode = match state.mode {
        MarketMode::Sell => MarketMode::Buy,
        MarketMode::Buy => MarketMode::Sell,
    };
    let new_mode = state.mode.clone();
    
    if let Some(price) = state.get_active_price() {
        copy_to_clipboard_internal(price);
    }
    
    let _ = app.emit("mode-changed", &new_mode);
    new_mode
}

#[tauri::command]
fn copy_price(price: f64) -> bool {
    copy_to_clipboard_internal(price)
}

fn copy_to_clipboard_internal(price: f64) -> bool {
    let formatted = format!("{:.2}", price);
    match ClipboardContext::new() {
        Ok(ctx) => ctx.set_text(formatted).is_ok(),
        Err(_) => false,
    }
}

#[tauri::command]
async fn fetch_type_info(type_id: i32) -> Option<esi::TypeInfo> {
    esi::get_type_info(type_id).await
}

#[tauri::command]
async fn fetch_price_history(type_id: i32) -> Vec<esi::MarketHistory> {
    esi::get_market_history(type_id).await.unwrap_or_default()
}

#[tauri::command]
fn refresh_market_data(app: AppHandle) -> MarketData {
    let data = market::parse_latest_log();
    let mut state = MARKET_STATE.lock();
    state.data = data.clone();
    
    if let Some(price) = state.get_active_price() {
        copy_to_clipboard_internal(price);
    }
    
    let _ = app.emit("market-updated", &data);
    data
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            #[cfg(not(target_os = "macos"))]
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_decorations(false);
                }
            }
            
            // Start file watcher in background
            std::thread::spawn(move || {
                watcher::start_watching(app_handle);
            });
            
            // Parse initial data
            let data = market::parse_latest_log();
            MARKET_STATE.lock().data = data;
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_market_data,
            get_current_mode,
            toggle_mode,
            copy_price,
            fetch_type_info,
            fetch_price_history,
            refresh_market_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
