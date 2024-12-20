use crate::ant::files::File;
use crate::ant::payments::PaymentOrderManager;
use tauri::{AppHandle, State};

mod ant;
pub mod logging;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn upload_files(
    app: AppHandle,
    files: Vec<File>,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), ()> {
    ant::files::upload_private_files_to_vault(app, files, payment_orders).await;
    Ok(())
}

#[tauri::command]
async fn upload_files_test(
    app: AppHandle,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), ()> {
    ant::files::payment_test(app, payment_orders).await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(PaymentOrderManager::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            upload_files,
            upload_files_test
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
