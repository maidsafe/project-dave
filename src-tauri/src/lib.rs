use crate::ant::files::File;
use crate::ant::payments::PaymentOrderManager;
use ant::files::{PrivateFileFromVault, PublicFileFromVault};
use autonomi::client::vault::VaultSecretKey;
use tauri::{AppHandle, State};

mod ant;

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
    ant::files::upload_files(app, files, "archive_name", payment_orders).await;
    Ok(())
}

#[tauri::command]
pub async fn get_private_files_from_vault(
    vault_key: &VaultSecretKey,
) -> Result<Vec<PrivateFileFromVault>, ()> {
    Ok(ant::files::get_private_files_from_vault(vault_key)
        .await
        .unwrap())
}

#[tauri::command]
pub async fn get_public_files_from_vault(
    vault_key: &VaultSecretKey,
) -> Result<Vec<PublicFileFromVault>, ()> {
    Ok(ant::files::get_public_files_from_vault(vault_key)
        .await
        .unwrap())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(PaymentOrderManager::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, upload_files])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
