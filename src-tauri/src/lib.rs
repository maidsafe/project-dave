use std::path::PathBuf;

use crate::ant::client::SharedClient;
use crate::ant::files::File;
use crate::ant::payments::{OrderID, OrderMessage, PaymentOrderManager};
use ant::{
    app_data::AppData,
    files::{FileFromVault, VaultStructure},
};
use autonomi::chunk::DataMapChunk;
use autonomi::client::data::DataAddress;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

mod ant;
pub mod logging;

#[derive(Serialize, Deserialize)]
pub struct AppStateInner {
    pub(crate) app_data: AppData,
}

#[derive(Debug, serde::Serialize)]
struct CommandError {
    message: String,
}

impl Default for AppStateInner {
    fn default() -> Self {
        Self {
            app_data: AppData::load()
                .inspect_err(|err| eprintln!("failed to load settings: {err:?}"))
                .unwrap_or_default(),
        }
    }
}
type AppState = Mutex<AppStateInner>;

#[tauri::command]
async fn app_data(state: State<'_, AppState>) -> Result<AppData, ()> {
    let state = state.lock().await;

    Ok(state.app_data.clone())
}

#[tauri::command]
async fn app_data_store(state: State<'_, AppState>, app_data: AppData) -> Result<(), ()> {
    let mut state = state.lock().await;

    println!("updating app data: {app_data:?}");
    state.app_data = app_data;
    state.app_data.store().map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn upload_files(
    app: AppHandle,
    files: Vec<File>,
    vault_key_signature: String,
    shared_client: State<'_, SharedClient>,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), ()> {
    let secret_key = autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    )
    .expect("Invalid vault key signature");

    ant::files::upload_private_files_to_vault(
        app,
        files,
        &secret_key,
        shared_client,
        payment_orders,
    )
    .await
    .map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn send_payment_order_message(
    id: OrderID,
    message: OrderMessage,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), ()> {
    payment_orders.send_order_message(id, message).await;
    Ok(())
}

#[tauri::command]
async fn get_vault_structure(
    vault_key_signature: String,
    shared_client: State<'_, SharedClient>,
) -> Result<VaultStructure, ()> {
    let secret_key = autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    )
    .expect("Invalid vault key signature");

    ant::files::get_vault_structure(&secret_key, shared_client)
        .await
        .map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn get_files_from_vault(
    vault_key_signature: String,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, ()> {
    let secret_key = autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    )
    .expect("Invalid vault key signature");

    ant::files::get_files_from_vault(&secret_key, shared_client)
        .await
        .map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn download_private_file(
    data_map: DataMapChunk,
    to_dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), CommandError> {
    ant::files::download_private_file(&data_map, to_dest, shared_client)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
}

#[tauri::command]
async fn download_public_file(
    addr: DataAddress,
    to_dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), ()> {
    ant::files::download_public_file(&addr, to_dest, shared_client)
        .await
        .map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn get_single_file_data(
    vault_key_signature: String,
    file_path: String,
    shared_client: State<'_, SharedClient>,
) -> Result<FileFromVault, ()> {
    ant::files::get_single_file_data(&vault_key_signature, &file_path, shared_client)
        .await
        .map_err(|_err| ()) // TODO: Map to serializable error
}

#[tauri::command]
async fn confirm_payment(
    order_id: u64,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), ()> {
    payment_orders.confirm_payment(order_id as u16).await;
    Ok(())
}

#[tauri::command]
async fn get_unique_download_path(downloads_path: String, filename: String) -> Result<String, ()> {
    use std::path::Path;

    let base_path = Path::new(&downloads_path);
    let file_path = base_path.join(&filename);

    // If file doesn't exist, return original path
    if !file_path.exists() {
        return Ok(file_path.to_string_lossy().to_string());
    }

    // Extract name and extension
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let extension = file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s))
        .unwrap_or_default();

    // Try numbered variants until we find one that doesn't exist
    for i in 1..1000 {
        let new_filename = format!("{} ({}){}", stem, i, extension);
        let new_path = base_path.join(&new_filename);
        if !new_path.exists() {
            return Ok(new_path.to_string_lossy().to_string());
        }
    }

    // Fallback if we can't find a unique name
    Err(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .manage(SharedClient::default())
        .manage(PaymentOrderManager::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            upload_files,
            send_payment_order_message,
            get_vault_structure,
            get_files_from_vault,
            download_private_file,
            download_public_file,
            get_single_file_data,
            confirm_payment,
            get_unique_download_path,
            app_data,
            app_data_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
