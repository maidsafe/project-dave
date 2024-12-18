use std::path::PathBuf;

use crate::ant::files::File;
use crate::ant::payments::PaymentOrderManager;
use ant::files::FileFromVault;
use autonomi::client::{
    data::{DataAddr, DataMapChunk},
    vault::VaultSecretKey,
};
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
async fn get_files_from_vault(vault_key: [u8; 32]) -> Result<Vec<FileFromVault>, ()> {
    let vault_key = VaultSecretKey::from_bytes(vault_key).unwrap();
    ant::files::get_files_from_vault(&vault_key)
        .await
        .map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn download_private_file(data_map: DataMapChunk, to_dest: PathBuf) -> Result<(), ()> {
    ant::files::download_private_file(data_map, to_dest)
        .await
        .map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn download_public_file(addr: DataAddr, to_dest: PathBuf) -> Result<(), ()> {
    ant::files::download_public_file(addr, to_dest)
        .await
        .map_err(|_err| ()) // TODO: Map to serializable error
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
            get_files_from_vault,
            download_private_file,
            download_public_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
