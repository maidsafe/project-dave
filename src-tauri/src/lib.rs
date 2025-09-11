use std::path::PathBuf;

use crate::ant::client::SharedClient;
use crate::ant::files::File;
use crate::ant::payments::{OrderID, OrderMessage, PaymentOrderManager};
use crate::ant::vault::VaultUpdate;
use ant::{
    app_data::AppData,
    files::{FileFromVault, VaultStructure},
    local_storage::LocalFileData,
};
use autonomi::chunk::DataMapChunk;
use autonomi::client::data::DataAddress;
use autonomi::client::quote::StoreQuote;
use autonomi::client::vault::VaultSecretKey;
use autonomi::Chunk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Removed unused rand import
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

mod ant;
pub mod logging;

pub enum PendingUploadData {
    SingleFile {
        file: File,
        datamap: DataMapChunk,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        secret_key: VaultSecretKey,
        add_to_vault: bool,
    },
    SingleFilePublic {
        file: File,
        datamap: DataMapChunk,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        add_to_vault: bool,
        vault_secret_key: Option<VaultSecretKey>,
    },
    PrivateArchive {
        files: Vec<File>,
        archive_name: String,
        archive_datamap: DataMapChunk,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        add_to_vault: bool,
        vault_secret_key: Option<VaultSecretKey>,
    },
    PublicArchive {
        files: Vec<File>,
        archive_name: String,
        archive_datamap: DataMapChunk,
        file_datamaps: Vec<DataMapChunk>,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        add_to_vault: bool,
        vault_secret_key: Option<VaultSecretKey>,
    },
}

#[derive(Default)]
pub struct PendingUploads {
    uploads: HashMap<String, PendingUploadData>,
}

impl PendingUploads {
    pub fn store_single_file(
        &mut self,
        upload_id: String,
        file: File,
        datamap: DataMapChunk,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        secret_key: VaultSecretKey,
        add_to_vault: bool,
    ) {
        self.uploads.insert(
            upload_id,
            PendingUploadData::SingleFile {
                file,
                datamap,
                chunks,
                store_quote,
                vault_update,
                secret_key,
                add_to_vault,
            },
        );
    }

    pub fn store_single_file_public(
        &mut self,
        upload_id: String,
        file: File,
        datamap: DataMapChunk,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        add_to_vault: bool,
        vault_secret_key: Option<VaultSecretKey>,
    ) {
        self.uploads.insert(
            upload_id,
            PendingUploadData::SingleFilePublic {
                file,
                datamap,
                chunks,
                store_quote,
                vault_update,
                add_to_vault,
                vault_secret_key,
            },
        );
    }

    pub fn store_public_archive(
        &mut self,
        upload_id: String,
        files: Vec<File>,
        archive_name: String,
        archive_datamap: DataMapChunk,
        file_datamaps: Vec<DataMapChunk>,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        add_to_vault: bool,
        vault_secret_key: Option<VaultSecretKey>,
    ) {
        self.uploads.insert(
            upload_id,
            PendingUploadData::PublicArchive {
                files,
                archive_name,
                archive_datamap,
                file_datamaps,
                chunks,
                store_quote,
                vault_update,
                add_to_vault,
                vault_secret_key,
            },
        );
    }

    pub fn store_private_archive(
        &mut self,
        upload_id: String,
        files: Vec<File>,
        archive_name: String,
        archive_datamap: DataMapChunk,
        chunks: Vec<Chunk>,
        store_quote: StoreQuote,
        vault_update: VaultUpdate,
        add_to_vault: bool,
        vault_secret_key: Option<VaultSecretKey>,
    ) {
        self.uploads.insert(
            upload_id,
            PendingUploadData::PrivateArchive {
                files,
                archive_name,
                archive_datamap,
                chunks,
                store_quote,
                vault_update,
                add_to_vault,
                vault_secret_key,
            },
        );
    }

    pub fn take(&mut self, upload_id: &str) -> Option<PendingUploadData> {
        self.uploads.remove(upload_id)
    }
}

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
type PendingUploadsState = Mutex<PendingUploads>;

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
async fn start_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: Option<String>,
    vault_key_signature: String,
    upload_id: String,  // Frontend provides the upload ID
    is_private: bool,   // New: privacy option
    add_to_vault: bool, // New: vault storage option
    shared_client: State<'_, SharedClient>,
    pending_uploads: State<'_, PendingUploadsState>,
) -> Result<(), CommandError> {
    // No need to return ID since frontend already has it

    // Determine vault secret key based on options
    let vault_secret_key = if add_to_vault {
        Some(
            autonomi::client::vault::key::vault_key_from_signature_hex(
                vault_key_signature.trim_start_matches("0x"),
            )
            .map_err(|e| CommandError {
                message: e.to_string(),
            })?,
        )
    } else {
        None
    };

    // Check if this is a single file upload (not a directory)
    let is_single_file = files.len() == 1 && {
        use std::fs;
        match fs::metadata(&files[0].path) {
            Ok(metadata) => metadata.is_file(),
            Err(_) => false,
        }
    };

    if is_single_file {
        if is_private {
            // Private file upload - always requires vault key for encryption
            let secret_key = vault_secret_key.ok_or_else(|| CommandError {
                message: "Private uploads require a vault signature".to_string(),
            })?;

            ant::files::start_private_single_file_upload(
                app,
                files.into_iter().next().unwrap(),
                &secret_key,
                upload_id.clone(),
                add_to_vault,
                shared_client,
                Some(&*pending_uploads),
            )
            .await
            .map_err(|e| CommandError {
                message: e.to_string(),
            })?;
        } else {
            // Public file upload - vault key is optional
            ant::files::start_public_single_file_upload(
                app,
                files.into_iter().next().unwrap(),
                upload_id.clone(),
                add_to_vault,
                vault_secret_key.as_ref(),
                shared_client,
                Some(&*pending_uploads),
            )
            .await
            .map_err(|e| CommandError {
                message: e.to_string(),
            })?;
        }
    } else {
        // Archive uploads - support both public and private archives
        let archive_name = archive_name.unwrap_or_default();

        if is_private {
            // Private archive upload - vault key is optional (only needed for add_to_vault)
            ant::files::start_private_archive_upload(
                app,
                files,
                archive_name,
                upload_id.clone(),
                add_to_vault,
                vault_secret_key.as_ref(),
                shared_client,
                Some(&*pending_uploads),
            )
            .await
            .map_err(|e| CommandError {
                message: e.to_string(),
            })?;
        } else {
            // Public archive upload - vault key is optional (only needed for add_to_vault)
            ant::files::start_public_archive_upload(
                app,
                files,
                archive_name,
                upload_id.clone(),
                add_to_vault,
                vault_secret_key.as_ref(),
                shared_client,
                Some(&*pending_uploads),
            )
            .await
            .map_err(|e| CommandError {
                message: e.to_string(),
            })?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn confirm_upload_payment(
    app: AppHandle,
    upload_id: String,
    shared_client: State<'_, SharedClient>,
    pending_uploads: State<'_, PendingUploadsState>,
) -> Result<(), CommandError> {
    let mut pending = pending_uploads.lock().await;

    if let Some(upload_data) = pending.take(&upload_id) {
        match upload_data {
            PendingUploadData::SingleFile {
                file,
                datamap,
                chunks,
                store_quote,
                vault_update,
                secret_key,
                add_to_vault,
            } => {
                let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);

                ant::files::execute_private_single_file_upload(
                    app,
                    file,
                    datamap,
                    chunks,
                    receipt,
                    vault_update,
                    &secret_key,
                    upload_id,
                    add_to_vault,
                    shared_client,
                )
                .await
                .map_err(|e| CommandError {
                    message: e.to_string(),
                })?;
            }
            PendingUploadData::SingleFilePublic {
                file,
                datamap,
                chunks,
                store_quote,
                vault_update,
                add_to_vault,
                vault_secret_key,
            } => {
                let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);

                ant::files::execute_public_single_file_upload(
                    app,
                    file,
                    datamap,
                    chunks,
                    receipt,
                    vault_update,
                    upload_id,
                    add_to_vault,
                    vault_secret_key.as_ref(),
                    shared_client,
                )
                .await
                .map_err(|e| CommandError {
                    message: e.to_string(),
                })?;
            }
            PendingUploadData::PublicArchive {
                files,
                archive_name,
                archive_datamap,
                file_datamaps,
                chunks,
                store_quote,
                add_to_vault,
                vault_update,
                vault_secret_key,
            } => {
                let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);

                ant::files::execute_public_archive_upload(
                    app,
                    files,
                    archive_name,
                    archive_datamap,
                    file_datamaps,
                    chunks,
                    receipt,
                    vault_update,
                    upload_id,
                    add_to_vault,
                    vault_secret_key.as_ref(),
                    shared_client,
                )
                .await
                .map_err(|e| CommandError {
                    message: e.to_string(),
                })?;
            }
            PendingUploadData::PrivateArchive {
                files,
                archive_name,
                archive_datamap,
                chunks,
                store_quote,
                vault_update,
                add_to_vault,
                vault_secret_key,
            } => {
                let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);

                ant::files::execute_private_archive_upload(
                    app,
                    files,
                    archive_name,
                    archive_datamap,
                    chunks,
                    receipt,
                    vault_update,
                    upload_id,
                    add_to_vault,
                    vault_secret_key.as_ref(),
                    shared_client,
                )
                .await
                .map_err(|e| CommandError {
                    message: e.to_string(),
                })?;
            }
        }
    } else {
        return Err(CommandError {
            message: format!("Upload {} not found or already processed", upload_id),
        });
    }

    Ok(())
}

// Cancel upload is only possible before payment - no backend command needed
// Frontend handles cancellation by not proceeding to execute_upload

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
async fn remove_from_vault(
    vault_key_signature: String,
    file_path: String,
    archive_address: Option<String>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), CommandError> {
    let secret_key = autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    )
    .expect("Invalid vault key signature");

    ant::files::remove_from_vault(&secret_key, &file_path, archive_address, shared_client)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
}

#[tauri::command]
async fn add_local_archive_to_vault(
    vault_key_signature: String,
    archive_address: String,
    archive_name: String,
    is_private: bool,
    shared_client: State<'_, SharedClient>,
) -> Result<(), CommandError> {
    eprintln!("=== add_local_archive_to_vault COMMAND ===");
    eprintln!("vault_key_signature: {}", vault_key_signature);
    eprintln!("archive_address: {}", archive_address);
    eprintln!("archive_name: {}", archive_name);
    eprintln!("is_private: {}", is_private);

    let secret_key = match autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    ) {
        Ok(key) => {
            eprintln!("Successfully parsed vault key");
            key
        }
        Err(e) => {
            eprintln!("Failed to parse vault key: {:?}", e);
            return Err(CommandError {
                message: format!("Invalid vault key signature: {:?}", e),
            });
        }
    };

    ant::files::add_local_archive_to_vault(
        &secret_key,
        &archive_address,
        &archive_name,
        is_private,
        shared_client,
    )
    .await
    .map_err(|err| {
        eprintln!("add_local_archive_to_vault failed with error: {:?}", err);
        CommandError {
            message: err.to_string(),
        }
    })
}

#[tauri::command]
async fn add_local_file_to_vault(
    vault_key_signature: String,
    file_address: String,
    file_name: String,
    is_private: bool,
    shared_client: State<'_, SharedClient>,
) -> Result<(), CommandError> {
    eprintln!("=== add_local_file_to_vault COMMAND ===");
    eprintln!("vault_key_signature: {}", vault_key_signature);
    eprintln!("file_address: {}", file_address);
    eprintln!("file_name: {}", file_name);
    eprintln!("is_private: {}", is_private);

    let secret_key = match autonomi::client::vault::key::vault_key_from_signature_hex(
        vault_key_signature.trim_start_matches("0x"),
    ) {
        Ok(key) => {
            eprintln!("Successfully parsed vault key");
            key
        }
        Err(e) => {
            eprintln!("Failed to parse vault key: {:?}", e);
            return Err(CommandError {
                message: format!("Invalid vault key signature: {:?}", e),
            });
        }
    };

    ant::files::add_local_file_to_vault(
        &secret_key,
        &file_address,
        &file_name,
        is_private,
        shared_client,
    )
    .await
    .map_err(|err| {
        eprintln!("add_local_file_to_vault failed with error: {:?}", err);
        CommandError {
            message: err.to_string(),
        }
    })
}

#[tauri::command]
async fn download_private_file(
    data_map_chunk: DataMapChunk,
    to_dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), CommandError> {
    ant::files::download_private_file(&data_map_chunk, to_dest, shared_client)
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
    ant::local_storage::get_all_local_files().map_err(|err| CommandError {
        message: err.to_string(),
    })
}

#[tauri::command]
async fn load_local_private_archive(
    local_addr: String,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, CommandError> {
    let client = shared_client
        .get_client()
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;

    let archive_datamap = ant::local_storage::get_local_private_archive_access(&local_addr)
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;

    let archive = client
        .archive_get(&archive_datamap)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;

    let mut files = Vec::new();
    for (filepath, (data_map, metadata)) in archive.map() {
        files.push(ant::files::FileFromVault::new(
            filepath.display().to_string(),
            metadata.clone(),
            ant::files::FileAccess::Private(data_map.clone()),
        ));
    }

    Ok(files)
}

#[tauri::command]
async fn load_local_public_archive(
    address_hex: String,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, CommandError> {
    let client = shared_client
        .get_client()
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;

    let archive_address = ant::local_storage::get_local_public_archive_address(&address_hex)
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;

    let archive = client
        .archive_get_public(&archive_address)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })?;

    let mut files = Vec::new();
    for (filepath, (data_addr, metadata)) in archive.map() {
        files.push(ant::files::FileFromVault::new(
            filepath.display().to_string(),
            metadata.clone(),
            ant::files::FileAccess::Public(*data_addr),
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
async fn delete_local_public_file(address: String) -> Result<(), CommandError> {
    ant::local_storage::delete_local_public_file(address)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
}

#[tauri::command]
async fn delete_local_private_file(address: String) -> Result<(), CommandError> {
    ant::local_storage::delete_local_private_file(address)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
}

#[tauri::command]
async fn delete_local_public_archive(address: String) -> Result<(), CommandError> {
    ant::local_storage::delete_local_public_archive(address)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
}

#[tauri::command]
async fn delete_local_private_archive(address: String) -> Result<(), CommandError> {
    ant::local_storage::delete_local_private_archive(address)
        .await
        .map_err(|err| CommandError {
            message: err.to_string(),
        })
}

#[tauri::command]
async fn get_local_private_file_access(
    local_addr: String,
) -> Result<serde_json::Value, CommandError> {
    let datamap =
        ant::local_storage::get_local_private_file_access(&local_addr).map_err(|err| {
            CommandError {
                message: err.to_string(),
            }
        })?;

    // Convert to JSON value for frontend
    serde_json::to_value(datamap).map_err(|err| CommandError {
        message: format!("Failed to serialize datamap: {}", err),
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
        .manage(PendingUploadsState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_upload,
            confirm_upload_payment,
            send_payment_order_message,
            get_vault_structure,
            get_vault_structure_streaming,
            get_files_from_vault,
            remove_from_vault,
            download_private_file,
            download_public_file,
            get_single_file_data,
            confirm_payment,
            get_unique_download_path,
            get_local_files,
            get_local_structure_streaming,
            load_local_private_archive,
            load_local_public_archive,
            add_local_archive_to_vault,
            add_local_file_to_vault,
            delete_local_public_file,
            delete_local_private_file,
            delete_local_public_archive,
            delete_local_private_archive,
            get_local_private_file_access,
            app_data,
            app_data_store,
            show_item_in_file_manager,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
