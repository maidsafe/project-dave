use std::path::PathBuf;

use crate::ant::files::File;
use crate::ant::payments::{OrderID, OrderMessage, PaymentOrderManager};
use ant::{app_data::AppData, files::FileFromVault};
use autonomi::{
    client::{
        data::{DataAddr, DataMapChunk},
        vault::VaultSecretKey,
    },
    Multiaddr,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

mod ant;
pub mod logging;

#[derive(Serialize, Deserialize)]
pub struct AppStateInner {
    pub(crate) app_data: AppData,
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
        .manage(AppState::default())
        .manage(PaymentOrderManager::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            upload_files,
            upload_files_test,
            send_payment_order_message,
            get_files_from_vault,
            download_private_file,
            download_public_file,
            app_data,
            app_data_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
