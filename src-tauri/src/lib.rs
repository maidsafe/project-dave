use std::path::PathBuf;

use crate::ant::client::SharedClient;
use crate::ant::files::File;
use crate::ant::payments::{OrderID, OrderMessage, PaymentOrderManager};
use ant::{
    app_data::AppData,
    files::{FileFromVault, VaultStructure},
    local_storage::LocalFileData,
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
    archive_name: Option<String>,
    vault_key_signature: String,
    shared_client: State<'_, SharedClient>,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), ()> {
    let secret_key = autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    )
    .expect("Invalid vault key signature");

    // Generate archive name if not provided
    let archive_name = archive_name.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        if files.len() == 1 {
            files[0].name.clone()
        } else {
            format!("{}_files_{}", files.len(), timestamp)
        }
    });

    ant::files::upload_private_files_to_vault(
        app,
        files,
        archive_name,
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
async fn get_vault_structure_streaming(
    app: AppHandle,
    vault_key_signature: String,
    temp_code: String,
    shared_client: State<'_, SharedClient>,
) -> Result<(), ()> {
    let secret_key = autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    )
    .expect("Invalid vault key signature");

    ant::files::get_vault_structure_streaming(app, &secret_key, temp_code, shared_client)
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
) -> Result<(), CommandError> {
    ant::files::download_public_file(&addr, to_dest, shared_client)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
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
async fn show_item_in_file_manager(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    
    match app.opener().reveal_item_in_dir(&path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to reveal item: {}", e)),
    }
}

#[tauri::command]
async fn get_local_files() -> Result<LocalFileData, CommandError> {
    ant::local_storage::get_all_local_files()
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
}

#[tauri::command]
async fn load_local_private_archive(
    local_addr: String,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, CommandError> {
    let client = shared_client.get_client().await.map_err(|err| CommandError {
        message: err.to_string(),
    })?;
    
    let archive_datamap = ant::local_storage::get_local_private_archive_access(&local_addr)
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;
    
    let archive = client.archive_get(&archive_datamap).await.map_err(|err| CommandError {
        message: err.to_string(),
    })?;
    
    let mut files = Vec::new();
    for (filepath, (data_map, metadata)) in archive.map() {
        files.push(ant::files::FileFromVault::new(
            filepath.display().to_string(),
            metadata.clone(),
            ant::files::PublicOrPrivateFile::Private(data_map.clone()),
        ));
    }
    
    Ok(files)
}

#[tauri::command]
async fn load_local_public_archive(
    address_hex: String,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, CommandError> {
    let client = shared_client.get_client().await.map_err(|err| CommandError {
        message: err.to_string(),
    })?;
    
    let archive_address = ant::local_storage::get_local_public_archive_address(&address_hex)
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;
    
    let archive = client.archive_get_public(&archive_address).await.map_err(|err| CommandError {
        message: err.to_string(),
    })?;
    
    let mut files = Vec::new();
    for (filepath, (data_addr, metadata)) in archive.map() {
        files.push(ant::files::FileFromVault::new(
            filepath.display().to_string(),
            metadata.clone(),
            ant::files::PublicOrPrivateFile::Public(*data_addr),
        ));
    }
    
    Ok(files)
}

#[tauri::command]
async fn get_local_structure_streaming(
    app: AppHandle,
    temp_code: String,
    shared_client: State<'_, SharedClient>,
) -> Result<(), CommandError> {
    ant::local_storage::get_local_structure_streaming(app, temp_code, shared_client)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            upload_files,
            send_payment_order_message,
            get_vault_structure,
            get_vault_structure_streaming,
            get_files_from_vault,
            download_private_file,
            download_public_file,
            get_single_file_data,
            confirm_payment,
            get_unique_download_path,
            get_local_files,
            get_local_structure_streaming,
            load_local_private_archive,
            load_local_public_archive,
            app_data,
            app_data_store,
            show_item_in_file_manager,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
