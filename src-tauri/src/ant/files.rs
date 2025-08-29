use crate::ant::client::SharedClient;
use crate::ant::local_storage;
// Removed unused imports from payments module
use autonomi::chunk::DataMapChunk;
use autonomi::client::quote::{DataTypes, StoreQuote};
use autonomi::client::vault::key::vault_key_from_signature_hex;
use autonomi::client::vault::{UserData, VaultSecretKey};
use autonomi::client::GetError;
use autonomi::data::DataAddress;
// Removed unused PrivateArchiveDataMap import
use autonomi::files::{Metadata, PrivateArchive};
use autonomi::vault::user_data::UserDataVaultError;
use autonomi::{Amount, Bytes, Chunk};
use hex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
// Removed unused Duration import
use tauri::{AppHandle, Emitter, State};
use thiserror::Error as ThisError;
use tokio::fs;
// Removed unused timeout import

#[derive(Deserialize, Clone)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum UploadProgress {
    Started {
        upload_id: String,
        total_files: usize,
        total_size: u64,
    },
    Processing {
        upload_id: String,
        current_file: String,
        files_processed: usize,
        total_files: usize,
        bytes_processed: u64,
        total_bytes: u64,
    },
    Encrypting {
        upload_id: String,
        current_file: String,
        files_processed: usize,
        total_files: usize,
    },
    RequestingPayment {
        upload_id: String,
        files_processed: usize,
        total_files: usize,
    },
    Uploading {
        upload_id: String,
        chunks_uploaded: usize,
        total_chunks: usize,
        bytes_uploaded: u64,
        total_bytes: u64,
    },
    Completed {
        upload_id: String,
        total_files: usize,
        total_bytes: u64,
    },
    Failed {
        upload_id: String,
        error: String,
    },
    Cancelled {
        upload_id: String,
    },
}

#[derive(ThisError, Debug)]
pub enum UploadError {
    #[error("Could not connect to the network: {0:?}")]
    Connect(#[from] autonomi::client::ConnectError),
    #[error("Could not read file: {0:?}")]
    Read(PathBuf),
    #[error("Failed to encrypt data: {0}")]
    Encryption(String),
    #[error("Failed to retrieve store quotes: {0}")]
    StoreQuote(String),
    #[error("Failed to get or create scratchpad: {0}")]
    Scratchpad(String),
    #[error("Failed to emit payment order: {0}")]
    EmitEvent(String),
}

#[derive(ThisError, Debug)]
pub enum VaultError {
    #[error("Could not connect to the network: {0:?}")]
    Connect(#[from] autonomi::client::ConnectError),
    #[error("Could not retrieve user data: {0:?}")]
    UserDataGet(#[from] UserDataVaultError),
    #[error("Could not retrieve data: {0:?}")]
    DataGet(#[from] GetError),
    #[error("File not found in vault")]
    FileNotFound,
}

#[derive(ThisError, Debug)]
pub enum DownloadError {
    #[error("Could not connect to the network: {0:?}")]
    Connect(#[from] autonomi::client::ConnectError),
    #[error("Could not download file: {0:?}")]
    Download(#[from] autonomi::client::files::DownloadError),
}

pub async fn read_file_to_bytes(file_path: PathBuf) -> Result<Bytes, UploadError> {
    tokio::fs::read(file_path.clone())
        .await
        .map(Bytes::from)
        .map_err(|_| UploadError::Read(file_path))
}

async fn calculate_total_size(files: &[File]) -> Result<u64, UploadError> {
    let mut total_size = 0u64;

    for file in files {
        let metadata = fs::metadata(&file.path)
            .await
            .map_err(|_| UploadError::Read(file.path.clone()))?;

        if metadata.is_dir() {
            // Calculate directory size
            let collected_files = collect_files_from_directory(file.path.clone()).await?;
            for (_, absolute_path) in collected_files {
                let file_metadata = fs::metadata(&absolute_path)
                    .await
                    .map_err(|_| UploadError::Read(absolute_path))?;
                total_size += file_metadata.len();
            }
        } else {
            total_size += metadata.len();
        }
    }

    Ok(total_size)
}

/// Collects files from a directory and its subdirectories, preserving relative paths.
/// Returns a vector of tuples containing (relative_path, absolute_path).
pub async fn collect_files_from_directory(
    dir_path: PathBuf,
) -> Result<Vec<(PathBuf, PathBuf)>, UploadError> {
    let mut files = Vec::new();
    let mut queue = VecDeque::new();

    // Get the parent directory to calculate relative paths correctly
    let base_dir = dir_path.parent().unwrap_or(&dir_path);
    let _dir_name = dir_path.file_name().unwrap_or_default();

    // Start with the root directory
    queue.push_back(dir_path.clone());

    while let Some(current_dir) = queue.pop_front() {
        let mut entries = fs::read_dir(&current_dir)
            .await
            .map_err(|_| UploadError::Read(current_dir.clone()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|_| UploadError::Read(current_dir.clone()))?
        {
            let path = entry.path();

            if path.is_dir() {
                // Add directory to the queue for processing
                queue.push_back(path);
            } else {
                // Calculate relative path including the root folder name
                if let Ok(rel_path) = path.strip_prefix(base_dir) {
                    files.push((rel_path.to_path_buf(), path));
                } else {
                    // Fallback if strip_prefix fails
                    files.push((path.clone(), path));
                }
            }
        }
    }

    Ok(files)
}

pub async fn start_single_file_upload(
    app: AppHandle,
    file: File,
    secret_key: &VaultSecretKey,
    upload_id: String,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    println!(
        ">>> start_single_file_upload called with upload_id: {}",
        upload_id
    );
    let client = shared_client.get_client().await.map_err(|e| {
        println!(">>> Failed to get client: {:?}", e);
        e
    })?;

    // Calculate file size
    let file_size = fs::metadata(&file.path)
        .await
        .map_err(|_| UploadError::Read(file.path.clone()))?
        .len();

    // Read and encrypt file to get chunks for quote
    println!(">>> Reading and encrypting file: {:?}", file.path);
    let bytes = read_file_to_bytes(file.path.clone()).await?;
    let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes).map_err(|err| {
        println!(">>> Encryption failed: {}", err);
        UploadError::Encryption(err.to_string())
    })?;
    println!(">>> File encrypted, got {} chunks", chunks.len());

    let chunks_iter = chunks
        .iter()
        .map(|chunk| (*chunk.address.xorname(), chunk.value.len()));

    println!(">>> Getting store quotes for {} chunks...", chunks.len());
    let store_quote = client
        .get_store_quotes(DataTypes::Chunk, chunks_iter)
        .await
        .map_err(|err| {
            println!(">>> Failed to get store quotes: {}", err);
            UploadError::StoreQuote(err.to_string())
        })?;
    println!(">>> Got store quote successfully");

    let total_cost: Amount = store_quote
        .payments()
        .iter()
        .map(|(_, _, amount)| *amount)
        .sum();
    let has_payments = total_cost > Amount::ZERO;

    // Emit quote event with cost information
    let payments: Vec<serde_json::Value> = if has_payments {
        store_quote
            .payments()
            .iter()
            .map(|(addr, _, amount)| {
                serde_json::json!({
                    "address": hex::encode(addr),
                    "amount": amount.to_string(),
                    "amount_formatted": format!("{} ATTO", amount)
                })
            })
            .collect()
    } else {
        vec![]
    };

    let raw_payments: Vec<_> = store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    println!(
        ">>> Emitting upload-quote event for upload_id: {}",
        upload_id
    );
    app.emit(
        "upload-quote",
        serde_json::json!({
            "upload_id": upload_id.clone(),
            "total_files": 1,
            "total_size": file_size,
            "total_cost_nano": total_cost.to_string(),
            "total_cost_formatted": format!("{} ATTO", total_cost),
            "payment_required": has_payments,
            "payments": payments,
            "raw_payments": raw_payments
        }),
    )
    .map_err(|err| {
        println!(">>> Failed to emit upload-quote event: {}", err);
        UploadError::EmitEvent(err.to_string())
    })?;
    println!(">>> Successfully emitted upload-quote event");

    // If no payment required, check if this is a duplicate (free) upload
    if !has_payments {
        // If cost is 0, this is a duplicate file - skip upload and mark as completed
        if total_cost == Amount::ZERO {
            println!(">>> Duplicate file detected (cost=0), marking as completed immediately for upload_id: {}", upload_id);
            // Emit completion immediately for duplicate files
            app.emit(
                "upload-progress",
                UploadProgress::Completed {
                    upload_id: upload_id.clone(),
                    total_files: 1,
                    total_bytes: file_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;
            println!(">>> Emitted completion event for duplicate file upload_id: {}", upload_id);
        } else {
            // Non-duplicate free upload - proceed normally
            execute_single_file_upload(
                app,
                file,
                DataMapChunk::from(datamap),
                chunks,
                store_quote,
                secret_key,
                upload_id,
                shared_client,
            )
            .await?;
        }
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_single_file(
            upload_id.clone(),
            file,
            DataMapChunk::from(datamap),
            chunks,
            store_quote,
            secret_key.clone(),
        );
    }
    // If payment required, the execution will happen when confirm_upload_payment is called

    Ok(())
}

pub async fn execute_single_file_upload(
    app: AppHandle,
    file: File,
    datamap: DataMapChunk,
    chunks: Vec<Chunk>,
    store_quote: StoreQuote,
    secret_key: &VaultSecretKey,
    upload_id: String,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    let client = shared_client.get_client().await?;
    let file_size = fs::metadata(&file.path)
        .await
        .map_err(|_| UploadError::Read(file.path.clone()))?
        .len();

    // Emit started event
    app.emit(
        "upload-progress",
        UploadProgress::Started {
            upload_id: upload_id.clone(),
            total_files: 1,
            total_size: file_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Emit uploading progress
    app.emit(
        "upload-progress",
        UploadProgress::Uploading {
            upload_id: upload_id.clone(),
            chunks_uploaded: 0,
            total_chunks: chunks.len(),
            bytes_uploaded: 0,
            total_bytes: file_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Spawn the actual upload work in background and return immediately
    let secret_key = secret_key.clone();
    tokio::spawn(async move {
        let result = async {
            // Upload chunks to network
            let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);
            client
                .chunk_batch_upload(chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            // Store file in vault
            let mut user_data = client
                .get_user_data_from_vault(&secret_key)
                .await
                .unwrap_or(UserData::new());

            // Add the single file to user data (not as archive)
            user_data
                .private_files
                .insert(DataMapChunk::from(datamap.clone()), file.name.clone());

            // Store updated user data in vault
            client
                .put_user_data_to_vault(&secret_key, receipt.into(), user_data)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            // Store file locally
            local_storage::write_local_private_file(datamap.to_hex(), datamap.address(), &file.name)
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            Ok::<(), UploadError>(())
        }.await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: 1,
                        total_bytes: file_size,
                    },
                ) {
                    eprintln!("Failed to emit completion event: {}", err);
                }
            }
            Err(err) => {
                // Emit failure
                if let Err(emit_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: err.to_string(),
                    },
                ) {
                    eprintln!("Failed to emit failure event: {}", emit_err);
                }
            }
        }
    });

    // Return immediately - the upload continues in background
    Ok(())
}

pub async fn start_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    secret_key: &VaultSecretKey,
    upload_id: String,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    let client = shared_client.get_client().await?;

    // Calculate total size and collect files
    let total_size = calculate_total_size(&files).await?;
    let total_files = files.len();

    // Create archive and encrypt to get chunks for quote
    let mut private_archive = PrivateArchive::new();
    let mut all_chunks = Vec::new();

    for file in &files {
        let path_metadata = fs::metadata(&file.path)
            .await
            .map_err(|_| UploadError::Read(file.path.clone()))?;

        if path_metadata.is_dir() {
            // Handle directory
            let collected_files = collect_files_from_directory(file.path.clone()).await?;
            for (relative_path, absolute_path) in collected_files {
                let bytes = read_file_to_bytes(absolute_path).await?;
                let file_size = bytes.len() as u64;
                let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes)
                    .map_err(|err| UploadError::Encryption(err.to_string()))?;

                all_chunks.extend(chunks);

                let metadata = Metadata::new_with_size(file_size);
                private_archive.add_file(relative_path, DataMapChunk::from(datamap), metadata);
            }
        } else {
            // Handle single file
            let bytes = read_file_to_bytes(file.path.clone()).await?;
            let file_size = bytes.len() as u64;
            let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes)
                .map_err(|err| UploadError::Encryption(err.to_string()))?;

            all_chunks.extend(chunks);

            let metadata = Metadata::new_with_size(file_size);
            private_archive.add_file(file.path.clone(), DataMapChunk::from(datamap), metadata);
        }
    }

    // Get store quote for all chunks
    let chunks_iter = all_chunks
        .iter()
        .map(|chunk| (*chunk.address.xorname(), chunk.value.len()));

    let store_quote = client
        .get_store_quotes(DataTypes::Chunk, chunks_iter)
        .await
        .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

    let total_cost: Amount = store_quote
        .payments()
        .iter()
        .map(|(_, _, amount)| *amount)
        .sum();
    let has_payments = total_cost > Amount::ZERO;

    // Emit quote event with cost information
    let payments: Vec<serde_json::Value> = if has_payments {
        store_quote
            .payments()
            .iter()
            .map(|(addr, _, amount)| {
                serde_json::json!({
                    "address": hex::encode(addr),
                    "amount": amount.to_string(),
                    "amount_formatted": format!("{} ATTO", amount)
                })
            })
            .collect()
    } else {
        vec![]
    };

    let raw_payments: Vec<_> = store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    app.emit(
        "upload-quote",
        serde_json::json!({
            "upload_id": upload_id.clone(),
            "total_files": total_files,
            "total_size": total_size,
            "total_cost_nano": total_cost.to_string(),
            "total_cost_formatted": format!("{} ATTO", total_cost),
            "payment_required": has_payments,
            "payments": payments,
            "raw_payments": raw_payments
        }),
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // If no payment required, check if this is a duplicate (free) upload
    if !has_payments {
        // If cost is 0, this is a duplicate file/archive - skip upload and mark as completed
        if total_cost == Amount::ZERO {
            println!(">>> Duplicate archive detected (cost=0), marking as completed immediately for upload_id: {}", upload_id);
            // Emit completion immediately for duplicate files
            app.emit(
                "upload-progress",
                UploadProgress::Completed {
                    upload_id: upload_id.clone(),
                    total_files: total_files,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;
            println!(">>> Emitted completion event for duplicate archive upload_id: {}", upload_id);
        } else {
            // Non-duplicate free upload - proceed normally
            execute_archive_upload(
                app,
                files,
                archive_name,
                private_archive,
                all_chunks,
                store_quote,
                secret_key,
                upload_id,
                shared_client,
            )
            .await?;
        }
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_archive(
            upload_id.clone(),
            files,
            archive_name,
            private_archive,
            all_chunks,
            store_quote,
            secret_key.clone(),
        );
    }
    // If payment required, the execution will happen when confirm_upload_payment is called

    Ok(())
}

pub async fn execute_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    private_archive: PrivateArchive,
    chunks: Vec<Chunk>,
    store_quote: StoreQuote,
    secret_key: &VaultSecretKey,
    upload_id: String,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    let client = shared_client.get_client().await?;
    let total_size = calculate_total_size(&files).await?;

    // Emit started event
    app.emit(
        "upload-progress",
        UploadProgress::Started {
            upload_id: upload_id.clone(),
            total_files: files.len(),
            total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Emit uploading progress
    app.emit(
        "upload-progress",
        UploadProgress::Uploading {
            upload_id: upload_id.clone(),
            chunks_uploaded: 0,
            total_chunks: chunks.len(),
            bytes_uploaded: 0,
            total_bytes: total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Spawn the actual upload work in background and return immediately
    let secret_key = secret_key.clone();
    let files_len = files.len();
    tokio::spawn(async move {
        let result = async {
            // Upload chunks to network
            let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);
            client
                .chunk_batch_upload(chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            // Store archive in vault
            let mut user_data = client
                .get_user_data_from_vault(&secret_key)
                .await
                .unwrap_or(UserData::new());

            // Store the archive
            let (archive_datamap, _archive_chunks) = autonomi::self_encryption::encrypt(
                private_archive
                    .to_bytes()
                    .map_err(|err| UploadError::Encryption(err.to_string()))?,
            )
            .map_err(|err| UploadError::Encryption(err.to_string()))?;

            user_data.private_file_archives.insert(
                DataMapChunk::from(archive_datamap.clone()),
                archive_name.clone(),
            );

            // Store updated user data in vault
            client
                .put_user_data_to_vault(&secret_key, receipt.into(), user_data)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            // Store archive locally
            let archive_datamap_chunk = DataMapChunk::from(archive_datamap);
            local_storage::write_local_private_file_archive(
                archive_datamap_chunk.to_hex(),
                archive_datamap_chunk.address(),
                &archive_name,
            )
            .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            Ok::<(), UploadError>(())
        }.await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: files_len,
                        total_bytes: total_size,
                    },
                ) {
                    eprintln!("Failed to emit completion event: {}", err);
                }
            }
            Err(err) => {
                // Emit failure
                if let Err(emit_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: err.to_string(),
                    },
                ) {
                    eprintln!("Failed to emit failure event: {}", emit_err);
                }
            }
        }
    });

    // Return immediately - the upload continues in background
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileFromVault {
    path: String,
    metadata: Metadata,
    file_access: PublicOrPrivateFile,
}

impl FileFromVault {
    pub fn new(path: String, metadata: Metadata, file_access: PublicOrPrivateFile) -> Self {
        Self {
            path,
            metadata,
            file_access,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FailedArchive {
    pub name: String,
    pub address: String,
    pub is_private: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStructure {
    pub archives: Vec<ArchiveInfo>,
    pub failed_archives: Vec<FailedArchive>,
    pub files: Vec<FileMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultUpdate {
    pub update_type: VaultUpdateType,
    pub archive: Option<ArchiveInfo>,
    pub failed_archive: Option<FailedArchive>,
    pub loading_archive: Option<LoadingArchive>,
    pub files: Vec<FileMetadata>,
    pub is_complete: bool,
    pub temp_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadingArchive {
    pub name: String,
    pub address: String,
    pub is_private: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VaultUpdateType {
    IndividualFiles,
    ArchiveLoading,
    ArchiveLoaded,
    ArchiveFailed,
    Complete,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchiveInfo {
    pub name: String,
    pub address: String,
    pub is_private: bool,
    pub files: Vec<FileMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub metadata: Metadata,
    pub file_type: FileType,
    pub is_loaded: bool,
    pub archive_name: String,
    pub access_data: Option<PublicOrPrivateFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FileType {
    Public,
    Private,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PublicOrPrivateFile {
    Public(DataAddress),
    Private(DataMapChunk),
}

pub async fn get_vault_structure(
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
) -> Result<VaultStructure, VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client.get_user_data_from_vault(secret_key).await?;

    let mut archives: Vec<ArchiveInfo> = vec![];
    let mut failed_archives: Vec<FailedArchive> = vec![];

    // Process private archives
    for (data_map, name) in user_data.private_file_archives {
        let archive_name = name.clone();

        if let Ok(archive) = client.archive_get(&data_map).await {
            let mut files: Vec<FileMetadata> = vec![];

            for (filepath, (data_map, metadata)) in archive.map() {
                let file = FileMetadata {
                    path: filepath.display().to_string(),
                    metadata: metadata.clone(),
                    file_type: FileType::Private,
                    is_loaded: false,
                    archive_name: archive_name.clone(),
                    access_data: Some(PublicOrPrivateFile::Private(data_map.clone())),
                };
                files.push(file);
            }

            archives.push(ArchiveInfo {
                name: archive_name.clone(),
                address: data_map.to_hex(),
                is_private: true,
                files,
            });
        } else {
            failed_archives.push(FailedArchive {
                name: archive_name.clone(),
                address: data_map.to_hex(),
                is_private: true,
            });
        }
    }

    // Process public archives
    for (archive_addr, name) in user_data.file_archives {
        let archive_name = name.clone();

        if let Ok(archive) = client.archive_get_public(&archive_addr).await {
            let mut files: Vec<FileMetadata> = vec![];

            for (filepath, (data_addr, metadata)) in archive.map() {
                let file = FileMetadata {
                    path: filepath.display().to_string(),
                    metadata: metadata.clone(),
                    file_type: FileType::Public,
                    is_loaded: false,
                    archive_name: archive_name.clone(),
                    access_data: Some(PublicOrPrivateFile::Public(*data_addr)),
                };
                files.push(file);
            }

            archives.push(ArchiveInfo {
                name: archive_name.clone(),
                address: archive_addr.to_hex(),
                is_private: false,
                files,
            });
        } else {
            failed_archives.push(FailedArchive {
                name: archive_name.clone(),
                address: archive_addr.to_hex(),
                is_private: false,
            });
        }
    }

    // Process individual files
    let mut individual_files: Vec<FileMetadata> = vec![];

    // Process individual private files
    for (data_map, name) in &user_data.private_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Private,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(PublicOrPrivateFile::Private(data_map.clone())),
        };
        individual_files.push(file);
    }

    // Process individual public files
    for (data_addr, name) in &user_data.public_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Public,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(PublicOrPrivateFile::Public(*data_addr)),
        };
        individual_files.push(file);
    }

    Ok(VaultStructure {
        archives,
        failed_archives,
        files: individual_files,
    })
}

pub async fn get_vault_structure_streaming(
    app: tauri::AppHandle,
    secret_key: &VaultSecretKey,
    temp_code: String,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client.get_user_data_from_vault(secret_key).await?;

    // First, emit individual files immediately (these are fast)
    let mut individual_files: Vec<FileMetadata> = vec![];

    // Process individual private files
    for (data_map, name) in &user_data.private_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Private,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(PublicOrPrivateFile::Private(data_map.clone())),
        };
        individual_files.push(file);
    }

    // Process individual public files
    for (data_addr, name) in &user_data.public_files {
        let file = FileMetadata {
            path: name.clone(),
            metadata: autonomi::files::Metadata::new_with_size(0),
            file_type: FileType::Public,
            is_loaded: true,
            archive_name: String::new(),
            access_data: Some(PublicOrPrivateFile::Public(*data_addr)),
        };
        individual_files.push(file);
    }

    // Emit individual files first if we have any
    if !individual_files.is_empty() {
        let update = VaultUpdate {
            update_type: VaultUpdateType::IndividualFiles,
            archive: None,
            failed_archive: None,
            loading_archive: None,
            files: individual_files,
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        app.emit("vault-update", update)
            .map_err(|_| VaultError::FileNotFound)?;
    }

    // Process archives concurrently
    let mut archive_tasks = vec![];

    // Create tasks for private archives
    for (data_map, name) in &user_data.private_file_archives {
        let client = client.clone();
        let app = app.clone();
        let archive_name = name.clone();
        let data_map = data_map.clone();
        
        // Emit loading status immediately
        let loading_update = VaultUpdate {
            update_type: VaultUpdateType::ArchiveLoading,
            archive: None,
            failed_archive: None,
            loading_archive: Some(LoadingArchive {
                name: archive_name.clone(),
                address: data_map.to_hex(),
                is_private: true,
            }),
            files: vec![],
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        let _ = app.emit("vault-update", loading_update);
        
        let temp_code = temp_code.clone();
        let task = tokio::spawn(async move {
            match client.archive_get(&data_map).await {
                Ok(archive) => {
                    let mut files: Vec<FileMetadata> = vec![];
                    
                    for (filepath, (data_map, metadata)) in archive.map() {
                        files.push(FileMetadata {
                            path: filepath.display().to_string(),
                            metadata: metadata.clone(),
                            file_type: FileType::Private,
                            is_loaded: true,
                            archive_name: archive_name.clone(),
                            access_data: Some(PublicOrPrivateFile::Private(data_map.clone())),
                        });
                    }

                    let archive_loaded = ArchiveInfo {
                        name: archive_name.clone(),
                        address: data_map.to_hex(),
                        is_private: true,
                        files,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveLoaded,
                        archive: Some(archive_loaded),
                        failed_archive: None,
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
                Err(_) => {
                    let failed_archive = FailedArchive {
                        name: archive_name.clone(),
                        address: data_map.to_hex(),
                        is_private: true,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveFailed,
                        archive: None,
                        failed_archive: Some(failed_archive),
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
            }
        });
        
        archive_tasks.push(task);
    }

    // Create tasks for public archives
    for (archive_addr, name) in &user_data.file_archives {
        let client = client.clone();
        let app = app.clone();
        let archive_name = name.clone();
        let archive_addr = *archive_addr;
        
        // Emit loading status immediately
        let loading_update = VaultUpdate {
            update_type: VaultUpdateType::ArchiveLoading,
            archive: None,
            failed_archive: None,
            loading_archive: Some(LoadingArchive {
                name: archive_name.clone(),
                address: archive_addr.to_hex(),
                is_private: false,
            }),
            files: vec![],
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        let _ = app.emit("vault-update", loading_update);
        
        let temp_code = temp_code.clone();
        let task = tokio::spawn(async move {
            match client.archive_get_public(&archive_addr).await {
                Ok(archive) => {
                    let mut files: Vec<FileMetadata> = vec![];
                    
                    for (filepath, (data_addr, metadata)) in archive.map() {
                        files.push(FileMetadata {
                            path: filepath.display().to_string(),
                            metadata: metadata.clone(),
                            file_type: FileType::Public,
                            is_loaded: true,
                            archive_name: archive_name.clone(),
                            access_data: Some(PublicOrPrivateFile::Public(data_addr.clone())),
                        });
                    }

                    let archive_loaded = ArchiveInfo {
                        name: archive_name.clone(),
                        address: archive_addr.to_hex(),
                        is_private: false,
                        files,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveLoaded,
                        archive: Some(archive_loaded),
                        failed_archive: None,
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
                Err(_) => {
                    let failed_archive = FailedArchive {
                        name: archive_name.clone(),
                        address: archive_addr.to_hex(),
                        is_private: false,
                    };

                    let update = VaultUpdate {
                        update_type: VaultUpdateType::ArchiveFailed,
                        archive: None,
                        failed_archive: Some(failed_archive),
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("vault-update", update);
                }
            }
        });
        
        archive_tasks.push(task);
    }

    // Wait for all archive tasks to complete
    for task in archive_tasks {
        let _ = task.await;
    }

    // Finally, emit completion
    let completion_update = VaultUpdate {
        update_type: VaultUpdateType::Complete,
        archive: None,
        failed_archive: None,
        loading_archive: None,
        files: vec![],
        is_complete: true,
        temp_code: temp_code.clone(),
    };
    app.emit("vault-update", completion_update)
        .map_err(|_| VaultError::FileNotFound)?;

    Ok(())
}

pub async fn get_files_from_vault(
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client.get_user_data_from_vault(secret_key).await?;

    let mut files: Vec<FileFromVault> = vec![];

    // Add individual private files
    for (data_map, name) in &user_data.private_files {
        let file = FileFromVault::new(
            name.clone(),
            autonomi::files::Metadata::new_with_size(0),
            PublicOrPrivateFile::Private(data_map.clone()),
        );
        files.push(file);
    }

    // Add individual public files
    for (data_addr, name) in &user_data.public_files {
        let file = FileFromVault::new(
            name.clone(),
            autonomi::files::Metadata::new_with_size(0),
            PublicOrPrivateFile::Public(*data_addr),
        );
        files.push(file);
    }

    Ok(files)
}

pub async fn download_private_file(
    data_map: &DataMapChunk,
    to_dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), DownloadError> {
    let client = shared_client.get_client().await?;
    let result = client.file_download(data_map, to_dest.clone()).await;

    // If download failed, clean up any zero-byte file that might have been created
    if result.is_err() && to_dest.exists() {
        if let Ok(metadata) = std::fs::metadata(&to_dest) {
            if metadata.len() == 0 {
                let _ = std::fs::remove_file(&to_dest);
            }
        }
    }

    result?;
    Ok(())
}

pub async fn download_public_file(
    addr: &DataAddress,
    to_dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), DownloadError> {
    let client = shared_client.get_client().await?;
    let result = client.file_download_public(addr, to_dest.clone()).await;

    // If download failed, clean up any zero-byte file that might have been created
    if result.is_err() && to_dest.exists() {
        if let Ok(metadata) = std::fs::metadata(&to_dest) {
            if metadata.len() == 0 {
                let _ = std::fs::remove_file(&to_dest);
            }
        }
    }

    result?;
    Ok(())
}

pub async fn get_single_file_data(
    vault_key_signature: &str,
    file_path: &str,
    shared_client: State<'_, SharedClient>,
) -> Result<FileFromVault, VaultError> {
    let secret_key = vault_key_from_signature_hex(vault_key_signature.trim_start_matches("0x"))
        .expect("Invalid vault key signature");
    let client = shared_client.get_client().await?;
    let user_data = client.get_user_data_from_vault(&secret_key).await?;

    // Try to find the file in individual private files
    for (data_map, name) in &user_data.private_files {
        if name == file_path {
            return Ok(FileFromVault::new(
                file_path.to_string(),
                autonomi::files::Metadata::new_with_size(0),
                PublicOrPrivateFile::Private(data_map.clone()),
            ));
        }
    }

    // Try to find the file in individual public files
    for (data_addr, name) in &user_data.public_files {
        if name == file_path {
            return Ok(FileFromVault::new(
                file_path.to_string(),
                autonomi::files::Metadata::new_with_size(0),
                PublicOrPrivateFile::Public(*data_addr),
            ));
        }
    }

    Err(VaultError::FileNotFound)
}
