use crate::ant::client::SharedClient;
use crate::ant::local_storage;
use autonomi::chunk::DataMapChunk;
use autonomi::client::quote::{DataTypes, StoreQuote};
use autonomi::client::vault::key::vault_key_from_signature_hex;
use autonomi::client::vault::{UserData, VaultSecretKey};
use autonomi::client::GetError;
use autonomi::data::DataAddress;
use autonomi::files::{Metadata, PrivateArchive, PublicArchive};
use autonomi::vault::user_data::UserDataVaultError;
use autonomi::{vault, Amount, Bytes, Chunk};
use hex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::VecDeque;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use thiserror::Error as ThisError;
use tokio::fs;

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
    #[error("Failed to serialize data: {0}")]
    Serialization(String),
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

pub async fn calculate_total_size(files: &[File]) -> Result<u64, UploadError> {
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

pub async fn start_private_single_file_upload(
    app: AppHandle,
    file: File,
    secret_key: &VaultSecretKey,
    upload_id: String,
    add_to_vault: bool,
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

    // If add_to_vault is true, get vault quote and add to total
    let mut total_store_quote = store_quote;
    if add_to_vault {
        println!(">>> Getting vault quote for add_to_vault...");
        // We'll need to create the vault data containing the file info
        let file_name = file.name.clone();

        let mut user_data = client
            .vault_get_user_data(&secret_key)
            .await
            .unwrap_or(UserData::new());

        user_data
            .private_files
            .insert(DataMapChunk::from(datamap.clone()), file_name);

        // Serialize user data to bytes for vault quote
        let vault_data = user_data
            .to_bytes()
            .map_err(|e| UploadError::Serialization(e.to_string()))?;

        // Get vault quote
        let vault_quote = crate::ant::vault::vault_quote(&client, vault_data, secret_key)
            .await
            .map_err(|e| UploadError::StoreQuote(e.to_string()))?;

        println!(">>> Got vault quote, merging with store quote");
        // Merge vault quote with store quote
        total_store_quote.0.extend(vault_quote.0);
    }

    let total_cost: Amount = total_store_quote
        .payments()
        .iter()
        .map(|(_, _, amount)| *amount)
        .sum();
    let has_payments = total_cost > Amount::ZERO;

    // Emit quote event with cost information
    let payments: Vec<serde_json::Value> = if has_payments {
        total_store_quote
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

    let raw_payments: Vec<_> = total_store_quote
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
            println!(
                ">>> Emitted completion event for duplicate file upload_id: {}",
                upload_id
            );
        } else {
            // Non-duplicate free upload - proceed normally
            execute_single_file_upload(
                app,
                file,
                DataMapChunk::from(datamap),
                chunks,
                total_store_quote,
                secret_key,
                upload_id,
                add_to_vault,
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
            total_store_quote,
            secret_key.clone(),
            add_to_vault,
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
    add_to_vault: bool,
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

            // Store file in vault if requested
            if add_to_vault {
                let mut user_data = client
                    .vault_get_user_data(&secret_key)
                    .await
                    .unwrap_or(UserData::new());

                // Add the single file to user data (not as archive)
                user_data
                    .private_files
                    .insert(DataMapChunk::from(datamap.clone()), file.name.clone());

                // Serialize user data for vault update
                let vault_data = user_data
                    .to_bytes()
                    .map_err(|e| UploadError::Serialization(e.to_string()))?;

                // Use vault_update with the receipt (which already includes vault payment)
                crate::ant::vault::vault_update(&client, vault_data, &secret_key, receipt)
                    .await
                    .map_err(|err| UploadError::StoreQuote(err.to_string()))?;
            }

            // Store file locally
            local_storage::write_local_private_file(
                datamap.to_hex(),
                datamap.address(),
                &file.name,
            )
            .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            Ok::<(), UploadError>(())
        }
        .await;

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

pub async fn start_private_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    println!(
        ">>> start_private_archive_upload called with upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
    );

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

    // Serialize and encrypt the archive itself
    let archive_bytes = private_archive
        .to_bytes()
        .map_err(|err| UploadError::Encryption(err.to_string()))?;
    let (archive_datamap, archive_chunks) = autonomi::self_encryption::encrypt(archive_bytes)
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    all_chunks.extend(archive_chunks);
    let archive_datamap_chunk = DataMapChunk::from(archive_datamap.clone());

    // Get store quote for all chunks (only file chunks, not archive datamap since it stays local)
    let chunks_iter = all_chunks
        .iter()
        .map(|chunk| (*chunk.address.xorname(), chunk.value.len()));

    println!(
        ">>> Getting store quotes for {} chunks...",
        all_chunks.len()
    );

    let store_quote = client
        .get_store_quotes(DataTypes::Chunk, chunks_iter)
        .await
        .map_err(|err| {
            println!(">>> Failed to get store quotes: {}", err);
            UploadError::StoreQuote(err.to_string())
        })?;

    println!(">>> Got store quote successfully");

    // If add_to_vault is true and vault_secret_key is provided, get vault quote and add to total
    let mut total_store_quote = store_quote;
    if add_to_vault && vault_secret_key.is_some() {
        let secret_key = vault_secret_key.unwrap();
        println!(">>> Getting vault quote for private archive add_to_vault...");
        // We'll need to create the vault data containing the archive info
        let archive_name_clone = archive_name.clone();

        // Create user data structure for this archive
        let mut user_data = client
            .vault_get_user_data(&secret_key)
            .await
            .unwrap_or(UserData::new());

        user_data
            .private_file_archives
            .insert(archive_datamap_chunk.clone(), archive_name_clone);

        // Serialize user data to bytes for vault quote
        let vault_data = user_data
            .to_bytes()
            .map_err(|e| UploadError::Serialization(e.to_string()))?;

        // Get vault quote
        let vault_quote = crate::ant::vault::vault_quote(&client, vault_data, secret_key)
            .await
            .map_err(|e| UploadError::StoreQuote(e.to_string()))?;

        println!(">>> Got vault quote for private archive, merging with store quote");
        // Merge vault quote with store quote
        total_store_quote.0.extend(vault_quote.0);
    }

    let total_cost: Amount = total_store_quote
        .payments()
        .iter()
        .map(|(_, _, amount)| *amount)
        .sum();
    let has_payments = total_cost > Amount::ZERO;

    // Emit quote event with cost information
    let payments: Vec<serde_json::Value> = if has_payments {
        total_store_quote
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

    let raw_payments: Vec<_> = total_store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    println!(
        ">>> Emitting upload-quote event for private archive upload_id: {}",
        upload_id
    );
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
    .map_err(|err| {
        println!(">>> Failed to emit upload-quote event: {}", err);
        UploadError::EmitEvent(err.to_string())
    })?;
    println!(">>> Successfully emitted upload-quote event");

    // If no payment required, proceed with upload
    if !has_payments {
        if total_cost == Amount::ZERO {
            println!(">>> Duplicate private archive detected (cost=0), marking as completed immediately for upload_id: {}", upload_id);
            // Emit completion immediately for duplicate archives
            app.emit(
                "upload-progress",
                UploadProgress::Completed {
                    upload_id: upload_id.clone(),
                    total_files: total_files,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;
            println!(
                ">>> Emitted completion event for duplicate private archive upload_id: {}",
                upload_id
            );
        } else {
            // Non-duplicate free upload - proceed normally
            execute_private_archive_upload(
                app,
                files,
                archive_name,
                archive_datamap_chunk,
                all_chunks,
                total_store_quote,
                upload_id,
                add_to_vault,
                vault_secret_key,
                shared_client,
            )
            .await?;
        }
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_private_archive(
            upload_id.clone(),
            files,
            archive_name,
            archive_datamap_chunk,
            all_chunks,
            total_store_quote,
            add_to_vault,
            vault_secret_key.cloned(),
        );
    }

    Ok(())
}

pub async fn execute_private_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    archive_datamap: DataMapChunk,
    chunks: Vec<Chunk>,
    store_quote: StoreQuote,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    println!(
        ">>> execute_private_archive_upload called for upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
    );

    let client = shared_client.get_client().await?;
    let total_size = calculate_total_size(&files).await?;

    // Emit upload started progress
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

    // Clone vault_secret_key for async closure
    let vault_secret_key = vault_secret_key.cloned();

    // Spawn the actual upload work in background
    tokio::spawn(async move {
        let result = async {
            // Upload all file chunks to network
            let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);

            client
                .chunk_batch_upload(chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            println!(">>> Uploaded {} chunks, datamap kept local", chunks.len());

            println!(
                ">>> Private archive uploaded successfully with local datamap: {:?}",
                archive_datamap.to_hex()
            );

            // Emit final uploading progress
            app.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    upload_id: upload_id.clone(),
                    chunks_uploaded: chunks.len(),
                    total_chunks: chunks.len(),
                    bytes_uploaded: total_size,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            // Add to vault if requested
            if add_to_vault {
                if let Some(secret_key) = vault_secret_key.as_ref() {
                    println!(">>> Adding private archive to vault...");

                    let mut user_data = client
                        .vault_get_user_data(&secret_key)
                        .await
                        .unwrap_or(UserData::new());

                    // Add the private archive to the vault using the archive datamap
                    user_data
                        .private_file_archives
                        .insert(archive_datamap.clone(), archive_name.clone());

                    // Serialize user data for vault update
                    let vault_data = user_data
                        .to_bytes()
                        .map_err(|e| UploadError::Serialization(e.to_string()))?;

                    // Use vault_update with the receipt (which already includes vault payment)
                    crate::ant::vault::vault_update(&client, vault_data, &secret_key, receipt)
                        .await
                        .map_err(|err| {
                            println!(">>> Failed to update vault: {:?}", err);
                            UploadError::Scratchpad(err.to_string())
                        })?;

                    println!(">>> Successfully added private archive to vault");
                } else {
                    println!(">>> Warning: add_to_vault=true but no vault_secret_key provided");
                }
            }

            // Store the archive locally for future reference
            local_storage::write_local_private_file_archive(
                archive_datamap.to_hex(),
                archive_datamap.address(),
                &archive_name,
            )
            .map_err(|err| {
                println!(">>> Warning: Failed to store local reference: {:?}", err);
                // Don't fail the upload for local storage issues
                err
            })
            .ok();

            Ok::<(), UploadError>(())
        }
        .await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: files.len(),
                        total_bytes: total_size,
                    },
                ) {
                    eprintln!("Failed to emit completion event: {}", err);
                }
            }
            Err(err) => {
                // Emit failure
                if let Err(_emit_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: err.to_string(),
                    },
                ) {
                    eprintln!("Failed to emit failure event: {}", err);
                }
            }
        }
    });

    // Return immediately - the upload continues in background
    Ok(())
}

pub async fn start_public_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    println!(
        ">>> start_public_archive_upload called with upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
    );

    let client = shared_client.get_client().await?;

    // Calculate total size and collect files
    let total_size = calculate_total_size(&files).await?;
    let total_files = files.len();

    // Create public archive - all files should be uploaded publicly
    let mut public_archive = PublicArchive::new(); // Structure is same, but we upload files publicly
    let mut all_chunks = Vec::new();
    let mut file_datamaps = Vec::new(); // Track file datamaps for public upload

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

                // For public archives, we need to upload each file's datamap publicly
                file_datamaps.push(DataMapChunk::from(datamap.clone()));

                // Use DataAddress (public file address) instead of DataMapChunk
                let metadata = Metadata::new_with_size(file_size);
                // We'll replace this with DataAddress after uploading the datamap
                public_archive.add_file(relative_path, DataAddress::new(*datamap.name()), metadata);
            }
        } else {
            // Handle single file
            let bytes = read_file_to_bytes(file.path.clone()).await?;
            let file_size = bytes.len() as u64;
            let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes)
                .map_err(|err| UploadError::Encryption(err.to_string()))?;

            all_chunks.extend(chunks);

            // For public archives, we need to upload each file's datamap publicly
            file_datamaps.push(DataMapChunk::from(datamap.clone()));

            let metadata = Metadata::new_with_size(file_size);
            public_archive.add_file(
                file.path.clone(),
                DataAddress::new(*datamap.name()),
                metadata,
            );
        }
    }

    // Serialize and encrypt the archive itself
    let archive_bytes = public_archive
        .to_bytes()
        .map_err(|err| UploadError::Encryption(err.to_string()))?;
    let (archive_datamap, archive_chunks) = autonomi::self_encryption::encrypt(archive_bytes)
        .map_err(|err| UploadError::Encryption(err.to_string()))?;

    all_chunks.extend(archive_chunks);
    let archive_datamap_chunk = DataMapChunk::from(archive_datamap.clone());

    // For public archives, we need quotes for:
    // 1. All file chunks
    // 2. All file datamaps (to upload them publicly)
    // 3. Archive chunks
    // 4. Archive datamap (to upload it publicly)
    let mut chunks_iter = all_chunks
        .iter()
        .map(|chunk| (*chunk.address.xorname(), chunk.value.len()))
        .collect::<Vec<_>>();

    // Add all file datamaps to quote calculation
    for file_datamap in &file_datamaps {
        chunks_iter.push((file_datamap.0.name().to_owned(), file_datamap.0.size()));
    }

    chunks_iter.push((archive_datamap.name().to_owned(), archive_datamap.size()));

    println!(
        ">>> Getting store quotes for {} file chunks + {} file datamaps + archive datamap...",
        all_chunks.len(),
        file_datamaps.len()
    );
    let store_quote = client
        .get_store_quotes(DataTypes::Chunk, chunks_iter.into_iter())
        .await
        .map_err(|err| {
            println!(">>> Failed to get store quotes: {}", err);
            UploadError::StoreQuote(err.to_string())
        })?;
    println!(">>> Got store quote successfully");

    // If add_to_vault is true and vault_secret_key is provided, get vault quote and add to total
    let mut total_store_quote = store_quote;
    if add_to_vault && vault_secret_key.is_some() {
        let secret_key = vault_secret_key.unwrap();
        println!(">>> Getting vault quote for public archive add_to_vault...");
        // We'll need to create the vault data containing the archive info
        let archive_name_clone = archive_name.clone();
        let archive_data_address = archive_datamap.address();

        // Create user data structure for this archive
        let mut user_data = client
            .vault_get_user_data(&secret_key)
            .await
            .unwrap_or(UserData::new());

        user_data.file_archives.insert(
            DataAddress::new(*archive_data_address.xorname()),
            archive_name_clone,
        );

        // Serialize user data to bytes for vault quote
        let vault_data = user_data
            .to_bytes()
            .map_err(|e| UploadError::Serialization(e.to_string()))?;

        // Get vault quote
        let vault_quote = crate::ant::vault::vault_quote(&client, vault_data, secret_key)
            .await
            .map_err(|e| UploadError::StoreQuote(e.to_string()))?;

        println!(">>> Got vault quote for public archive, merging with store quote");
        // Merge vault quote with store quote
        total_store_quote.0.extend(vault_quote.0);
    }

    let total_cost: Amount = total_store_quote
        .payments()
        .iter()
        .map(|(_, _, amount)| *amount)
        .sum();
    let has_payments = total_cost > Amount::ZERO;

    // Emit quote event with cost information
    let payments: Vec<serde_json::Value> = if has_payments {
        total_store_quote
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

    let raw_payments: Vec<_> = total_store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    println!(
        ">>> Emitting upload-quote event for public archive upload_id: {}",
        upload_id
    );
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
    .map_err(|err| {
        println!(">>> Failed to emit upload-quote event: {}", err);
        UploadError::EmitEvent(err.to_string())
    })?;
    println!(">>> Successfully emitted upload-quote event");

    // If no payment required, proceed with upload
    if !has_payments {
        if total_cost == Amount::ZERO {
            println!(">>> Duplicate public archive detected (cost=0), marking as completed immediately for upload_id: {}", upload_id);
            // Emit completion immediately for duplicate archives
            app.emit(
                "upload-progress",
                UploadProgress::Completed {
                    upload_id: upload_id.clone(),
                    total_files: total_files,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;
            println!(
                ">>> Emitted completion event for duplicate public archive upload_id: {}",
                upload_id
            );
        } else {
            // Non-duplicate free upload - proceed normally
            execute_public_archive_upload(
                app,
                files,
                archive_name,
                archive_datamap_chunk,
                file_datamaps,
                all_chunks,
                total_store_quote,
                upload_id,
                add_to_vault,
                vault_secret_key,
                shared_client,
            )
            .await?;
        }
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_public_archive(
            upload_id.clone(),
            files,
            archive_name,
            archive_datamap_chunk,
            file_datamaps,
            all_chunks,
            total_store_quote,
            add_to_vault,
            vault_secret_key.cloned(),
        );
    }

    Ok(())
}

pub async fn execute_public_archive_upload(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    archive_datamap: DataMapChunk,
    file_datamaps: Vec<DataMapChunk>,
    chunks: Vec<Chunk>,
    store_quote: StoreQuote,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    println!(
        ">>> execute_public_archive_upload called for upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
    );

    let client = shared_client.get_client().await?;
    let total_size = calculate_total_size(&files).await?;

    // Emit upload started progress
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
            total_chunks: chunks.len() + file_datamaps.len() + 1, // +file datamaps +1 for archive datamap
            bytes_uploaded: 0,
            total_bytes: total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Clone vault_secret_key for async closure
    let vault_secret_key = vault_secret_key.cloned();

    // Spawn the actual upload work in background
    tokio::spawn(async move {
        let result = async {
            // Upload all file chunks to network
            let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);
            client
                .chunk_batch_upload(chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            // For public archives: Upload all file datamaps and archive datamap to network
            // This is the key difference from private archives!

            // Upload all file datamaps
            let mut datamap_chunks = Vec::new();
            for file_datamap_chunk in file_datamaps {
                datamap_chunks.push(file_datamap_chunk.0);
            }

            datamap_chunks.push(archive_datamap.0.clone());

            // Upload all datamaps using the same receipt since we included them in the quote
            client
                .chunk_batch_upload(datamap_chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            println!(
                ">>> Uploaded {} file chunks + {} datamaps (including archive datamap)",
                chunks.len(),
                datamap_chunks.len()
            );

            // The public archive's address is the archive datamap's address
            let public_archive_address = DataAddress::new(archive_datamap.0.name().to_owned());

            println!(
                ">>> Public archive uploaded successfully with address: {:?}",
                public_archive_address
            );

            // Emit final uploading progress
            app.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    upload_id: upload_id.clone(),
                    chunks_uploaded: chunks.len() + 1,
                    total_chunks: chunks.len() + 1,
                    bytes_uploaded: total_size,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            // Add to vault if requested
            if add_to_vault {
                if let Some(secret_key) = vault_secret_key.as_ref() {
                    println!(">>> Adding public archive to vault...");

                    let mut user_data = client
                        .vault_get_user_data(&secret_key)
                        .await
                        .unwrap_or(UserData::new());

                    // Add the public archive to the vault using the archive datamap address
                    user_data
                        .file_archives
                        .insert(public_archive_address, archive_name.clone());

                    // Serialize user data for vault update
                    let vault_data = user_data
                        .to_bytes()
                        .map_err(|e| UploadError::Serialization(e.to_string()))?;

                    // Use vault_update with the receipt (which already includes vault payment)
                    crate::ant::vault::vault_update(&client, vault_data, &secret_key, receipt)
                        .await
                        .map_err(|err| {
                            println!(">>> Failed to update vault: {:?}", err);
                            UploadError::Scratchpad(err.to_string())
                        })?;

                    println!(">>> Successfully added public archive to vault");
                } else {
                    println!(">>> Warning: add_to_vault=true but no vault_secret_key provided");
                }
            }

            // Store the archive locally for future reference
            local_storage::write_local_public_file_archive(
                hex::encode(public_archive_address.xorname().0),
                &archive_name,
            )
            .map_err(|err| {
                println!(">>> Warning: Failed to store local reference: {:?}", err);
                // Don't fail the upload for local storage issues
                err
            })
            .ok();

            Ok::<(), UploadError>(())
        }
        .await;

        match result {
            Ok(()) => {
                // Emit completion
                if let Err(err) = app.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        upload_id: upload_id.clone(),
                        total_files: files.len(),
                        total_bytes: total_size,
                    },
                ) {
                    eprintln!("Failed to emit completion event: {}", err);
                }
            }
            Err(err) => {
                // Emit failure
                if let Err(_emit_err) = app.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        upload_id: upload_id.clone(),
                        error: err.to_string(),
                    },
                ) {
                    eprintln!("Failed to emit failure event: {}", err);
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

pub async fn remove_from_vault(
    secret_key: &VaultSecretKey,
    file_path: &str,
    archive_address: Option<String>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;

    // Get current user data from vault
    let mut user_data = client.get_user_data_from_vault(secret_key).await?;

    // If archive_address is provided, remove the entire archive
    if let Some(ref address) = archive_address {
        // Try to remove from private archives first
        let mut found = false;

        // Remove from private_file_archives by address
        user_data.private_file_archives.retain(|data_map, _name| {
            if data_map.to_hex() == *address {
                found = true;
                false // Remove this archive
            } else {
                true // Keep this archive
            }
        });

        // If not found in private archives, try public archives
        if !found {
            user_data.file_archives.retain(|data_addr, _name| {
                if data_addr.to_hex() == *address {
                    found = true;
                    false // Remove this archive
                } else {
                    true // Keep this archive
                }
            });
        }

        if !found {
            return Err(VaultError::FileNotFound);
        }
    } else {
        // Remove individual file by path
        let mut found = false;

        // Try to remove from private files first
        user_data.private_files.retain(|_data_map, name| {
            if name == file_path {
                found = true;
                false // Remove this file
            } else {
                true // Keep this file
            }
        });

        // If not found in private files, try public files
        if !found {
            user_data.public_files.retain(|_data_addr, name| {
                if name == file_path {
                    found = true;
                    false // Remove this file
                } else {
                    true // Keep this file
                }
            });
        }

        if !found {
            return Err(VaultError::FileNotFound);
        }
    }

    // Save the updated user data back to vault (vault updates are free)
    // Create an empty receipt for free vault operations
    let empty_store_quote = client
        .get_store_quotes(DataTypes::Chunk, std::iter::empty())
        .await
        .map_err(|_err| VaultError::FileNotFound)?;
    let receipt = autonomi::client::payment::receipt_from_store_quotes(empty_store_quote);

    client
        .put_user_data_to_vault(secret_key, receipt.into(), user_data)
        .await
        .map_err(|_err| VaultError::FileNotFound)?;
    Ok(())
}

pub async fn add_local_archive_to_vault(
    secret_key: &VaultSecretKey,
    archive_address: &str,
    archive_name: &str,
    is_private: bool,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;

    // Debug logging
    eprintln!("=== ADD LOCAL ARCHIVE TO VAULT DEBUG ===");
    eprintln!("archive_address: {}", archive_address);
    eprintln!("archive_name: {}", archive_name);
    eprintln!("is_private: {}", is_private);

    // Get current user data from vault
    let mut user_data = match client.get_user_data_from_vault(secret_key).await {
        Ok(data) => {
            eprintln!("Successfully retrieved user data from vault");
            data
        }
        Err(e) => {
            eprintln!("Failed to get user data from vault: {:?}", e);
            // Check if this is a case where the vault doesn't exist yet
            match &e {
                UserDataVaultError::GetError(_) => {
                    eprintln!("Vault might not exist yet, creating new user data");
                    // Create new user data if vault doesn't exist
                    UserData::new()
                }
                UserDataVaultError::Vault(_) => {
                    eprintln!("Vault error, might not exist yet, creating new user data");
                    // Create new user data if vault doesn't exist
                    UserData::new()
                }
                _ => {
                    eprintln!("Other vault error, returning error");
                    return Err(VaultError::UserDataGet(e));
                }
            }
        }
    };

    if is_private {
        // Resolve local address to actual network DataMapChunk
        eprintln!(
            "Resolving local private archive address: {}",
            archive_address
        );
        let data_map = match local_storage::get_local_private_archive_access(archive_address) {
            Ok(dm) => {
                eprintln!("Successfully resolved local address to DataMapChunk");
                dm
            }
            Err(e) => {
                eprintln!(
                    "Failed to resolve local private archive address '{}': {:?}",
                    archive_address, e
                );
                return Err(VaultError::FileNotFound);
            }
        };

        // Add to private archives using the resolved network address
        user_data
            .private_file_archives
            .insert(data_map, archive_name.to_string());
        eprintln!(
            "Added to private archives: {} -> {}",
            archive_address, archive_name
        );
    } else {
        // Resolve local address to actual network DataAddress
        eprintln!(
            "Resolving local public archive address: {}",
            archive_address
        );
        let data_addr = match local_storage::get_local_public_archive_address(archive_address) {
            Ok(addr) => {
                eprintln!("Successfully resolved local address to DataAddress");
                addr
            }
            Err(e) => {
                eprintln!(
                    "Failed to resolve local public archive address '{}': {:?}",
                    archive_address, e
                );
                return Err(VaultError::FileNotFound);
            }
        };

        // Add to public archives using the resolved network address
        user_data
            .file_archives
            .insert(data_addr, archive_name.to_string());
        eprintln!(
            "Added to public archives: {} -> {}",
            archive_address, archive_name
        );
    }

    // Save the updated user data back to vault (vault updates are free)
    // Create an empty receipt for free vault operations
    let empty_store_quote = match client
        .get_store_quotes(DataTypes::Chunk, std::iter::empty())
        .await
    {
        Ok(quote) => quote,
        Err(e) => {
            eprintln!("Failed to get store quotes: {:?}", e);
            return Err(VaultError::FileNotFound);
        }
    };
    let receipt = autonomi::client::payment::receipt_from_store_quotes(empty_store_quote);

    match client
        .put_user_data_to_vault(secret_key, receipt.into(), user_data)
        .await
    {
        Ok(_) => {
            eprintln!("Successfully updated vault with new archive");
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to put user data to vault: {:?}", e);
            Err(VaultError::FileNotFound)
        }
    }
}

pub async fn add_local_file_to_vault(
    secret_key: &VaultSecretKey,
    file_address: &str,
    file_name: &str,
    is_private: bool,
    shared_client: State<'_, SharedClient>,
) -> Result<(), VaultError> {
    let client = shared_client.get_client().await?;

    // Debug logging
    eprintln!("=== ADD LOCAL FILE TO VAULT DEBUG ===");
    eprintln!("file_address: {}", file_address);
    eprintln!("file_name: {}", file_name);
    eprintln!("is_private: {}", is_private);

    // Get current user data from vault
    let mut user_data = match client.get_user_data_from_vault(secret_key).await {
        Ok(data) => {
            eprintln!("Successfully retrieved user data from vault");
            data
        }
        Err(e) => {
            eprintln!("Failed to get user data from vault: {:?}", e);
            // Check if this is a case where the vault doesn't exist yet
            match &e {
                UserDataVaultError::GetError(_) => {
                    eprintln!("Vault might not exist yet, creating new user data");
                    UserData::new()
                }
                UserDataVaultError::Vault(_) => {
                    eprintln!("Vault error, might not exist yet, creating new user data");
                    UserData::new()
                }
                _ => {
                    eprintln!("Other vault error, returning error");
                    return Err(VaultError::UserDataGet(e));
                }
            }
        }
    };

    // Use the network address directly (passed from frontend)
    if is_private {
        eprintln!("Parsing private file hex network address: {}", file_address);
        let data_map = match DataMapChunk::from_hex(file_address) {
            Ok(dm) => {
                eprintln!("Successfully parsed private file address to DataMapChunk");
                dm
            }
            Err(e) => {
                eprintln!(
                    "Failed to parse private file address '{}': {:?}",
                    file_address, e
                );
                return Err(VaultError::FileNotFound);
            }
        };

        // Add to private files using the network address
        user_data
            .private_files
            .insert(data_map, file_name.to_string());
        eprintln!("Added private file to vault: {}", file_name);
    } else {
        eprintln!("Parsing public file hex network address: {}", file_address);
        let data_addr = match DataAddress::from_hex(file_address) {
            Ok(addr) => {
                eprintln!("Successfully parsed public file address to DataAddress");
                addr
            }
            Err(e) => {
                eprintln!(
                    "Failed to parse public file address '{}': {:?}",
                    file_address, e
                );
                return Err(VaultError::FileNotFound);
            }
        };

        // Add to public files using the network address
        user_data
            .public_files
            .insert(data_addr, file_name.to_string());
        eprintln!("Added public file to vault: {}", file_name);
    }

    // Save the updated user data back to vault (vault updates are free)
    // Create an empty receipt for free vault operations
    let empty_store_quote = match client
        .get_store_quotes(DataTypes::Chunk, std::iter::empty())
        .await
    {
        Ok(quote) => quote,
        Err(e) => {
            eprintln!("Failed to get store quotes: {:?}", e);
            return Err(VaultError::FileNotFound);
        }
    };
    let receipt = autonomi::client::payment::receipt_from_store_quotes(empty_store_quote);

    match client
        .put_user_data_to_vault(secret_key, receipt.into(), user_data)
        .await
    {
        Ok(_) => {
            eprintln!("Successfully updated vault with new file");
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to put user data to vault: {:?}", e);
            Err(VaultError::FileNotFound)
        }
    }
}

pub async fn start_public_single_file_upload(
    app: AppHandle,
    file: File,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
    pending_uploads: Option<&tokio::sync::Mutex<crate::PendingUploads>>,
) -> Result<(), UploadError> {
    println!(
        ">>> start_single_file_upload_public called with upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
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

    // Read and encrypt file to get chunks for quote (same as private files)
    println!(">>> Reading and encrypting public file: {:?}", file.path);
    let bytes = read_file_to_bytes(file.path.clone()).await?;
    let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes).map_err(|err| {
        println!(">>> Encryption failed: {}", err);
        UploadError::Encryption(err.to_string())
    })?;
    let datamap_chunk = DataMapChunk::from(datamap.clone());
    println!(">>> File encrypted, got {} chunks", chunks.len());

    let mut quote_iter = chunks
        .iter()
        .map(|chunk| (*chunk.address.xorname(), chunk.value.len()))
        .collect::<Vec<_>>();

    // Add datamap to quote calculation
    quote_iter.push((datamap.name().to_owned(), datamap.size()));

    println!(
        ">>> Getting store quotes for {} chunks + datamap...",
        chunks.len()
    );
    let store_quote = client
        .get_store_quotes(DataTypes::Chunk, quote_iter.into_iter())
        .await
        .map_err(|err| {
            println!(">>> Failed to get store quotes: {}", err);
            UploadError::StoreQuote(err.to_string())
        })?;
    println!(">>> Got store quote successfully");

    // If add_to_vault is true and vault_secret_key is provided, get vault quote and add to total
    let mut total_store_quote = store_quote;
    if add_to_vault && vault_secret_key.is_some() {
        let secret_key = vault_secret_key.unwrap();
        println!(">>> Getting vault quote for public file add_to_vault...");
        // We'll need to create the vault data containing the file info
        let file_name = file.name.clone();
        let data_address = datamap.address();

        // Create user data structure for this file
        let mut user_data = client
            .vault_get_user_data(&secret_key)
            .await
            .unwrap_or(UserData::new());

        user_data
            .public_files
            .insert(DataAddress::new(*data_address.xorname()), file_name);

        // Serialize user data to bytes for vault quote
        let vault_data = user_data
            .to_bytes()
            .map_err(|e| UploadError::Serialization(e.to_string()))?;

        // Get vault quote
        let vault_quote = crate::ant::vault::vault_quote(&client, vault_data, secret_key)
            .await
            .map_err(|e| UploadError::StoreQuote(e.to_string()))?;

        println!(">>> Got vault quote for public file, merging with store quote");
        // Merge vault quote with store quote
        total_store_quote.0.extend(vault_quote.0);
    }

    let total_cost: Amount = total_store_quote
        .payments()
        .iter()
        .map(|(_, _, amount)| *amount)
        .sum();
    let has_payments = total_cost > Amount::ZERO;

    // Emit quote event with cost information
    let payments: Vec<serde_json::Value> = if has_payments {
        total_store_quote
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

    let raw_payments: Vec<_> = total_store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    println!(
        ">>> Emitting upload-quote event for public upload_id: {}",
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

    // If no payment required, proceed with upload
    if !has_payments {
        if total_cost == Amount::ZERO {
            println!(">>> Duplicate public file detected (cost=0), marking as completed immediately for upload_id: {}", upload_id);
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
            println!(
                ">>> Emitted completion event for duplicate public file upload_id: {}",
                upload_id
            );
        } else {
            // Non-duplicate free upload - proceed normally
            execute_single_file_upload_public(
                app,
                file,
                datamap_chunk,
                chunks,
                total_store_quote,
                upload_id,
                add_to_vault,
                vault_secret_key,
                shared_client,
            )
            .await?;
        }
    } else if let Some(pending_uploads) = pending_uploads {
        // Store upload data for later execution after payment
        let mut pending = pending_uploads.lock().await;
        pending.store_single_file_public(
            upload_id.clone(),
            file,
            datamap_chunk,
            chunks,
            total_store_quote,
            add_to_vault,
            vault_secret_key.cloned(),
        );
    }

    Ok(())
}

pub async fn execute_single_file_upload_public(
    app: AppHandle,
    file: File,
    datamap: DataMapChunk,
    chunks: Vec<Chunk>,
    store_quote: StoreQuote,
    upload_id: String,
    add_to_vault: bool,
    vault_secret_key: Option<&VaultSecretKey>,
    shared_client: State<'_, SharedClient>,
) -> Result<(), UploadError> {
    println!(
        ">>> execute_single_file_upload_public called for upload_id: {}, add_to_vault: {}",
        upload_id, add_to_vault
    );

    let client = shared_client.get_client().await?;
    let file_size = fs::metadata(&file.path)
        .await
        .map_err(|_| UploadError::Read(file.path.clone()))?
        .len();

    // Emit upload started progress
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
            total_chunks: chunks.len() + 1, // +1 for datamap
            bytes_uploaded: 0,
            total_bytes: file_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    // Clone vault_secret_key for async closure
    let vault_secret_key = vault_secret_key.cloned();

    // Spawn the actual upload work in background
    tokio::spawn(async move {
        let result = async {
            // Upload file chunks to network (same as private files)
            let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);
            client
                .chunk_batch_upload(chunks.iter().collect(), &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            // The datamap upload uses the same receipt as the chunks since we included it in the quote
            client
                .chunk_batch_upload(vec![&datamap.0], &receipt)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            println!(">>> Uploaded {} file chunks + datamap chunk", chunks.len());

            // The public file's address is the datamap's address (not the file chunks)
            let public_data_address = DataAddress::new(*datamap.0.name());

            println!(
                ">>> Public file uploaded successfully with address: {:?}",
                public_data_address
            );

            // Emit final uploading progress
            app.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    upload_id: upload_id.clone(),
                    chunks_uploaded: chunks.len() + 1, // +1 for datamap
                    total_chunks: chunks.len() + 1,
                    bytes_uploaded: file_size,
                    total_bytes: file_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            // Add to vault if requested
            if add_to_vault {
                if let Some(secret_key) = vault_secret_key.as_ref() {
                    println!(">>> Adding public file to vault...");

                    let file_name = file.name.clone();

                    let mut user_data = client
                        .vault_get_user_data(&secret_key)
                        .await
                        .unwrap_or(UserData::new());

                    // Add the public file to the vault using the datamap address
                    user_data
                        .public_files
                        .insert(public_data_address, file_name.clone());

                    // Serialize user data for vault update
                    let vault_data = user_data
                        .to_bytes()
                        .map_err(|e| UploadError::Serialization(e.to_string()))?;

                    // Use vault_update with the receipt (which already includes vault payment)
                    crate::ant::vault::vault_update(&client, vault_data, &secret_key, receipt)
                        .await
                        .map_err(|err| {
                            println!(">>> Failed to update vault: {:?}", err);
                            UploadError::Scratchpad(err.to_string())
                        })?;

                    println!(">>> Successfully added public file to vault");
                } else {
                    println!(">>> Warning: add_to_vault=true but no vault_secret_key provided");
                }
            }

            // Store the file locally for future reference
            local_storage::write_local_public_file(
                hex::encode(public_data_address.xorname().0),
                &file.name,
            )
            .map_err(|err| {
                println!(">>> Warning: Failed to store local reference: {:?}", err);
                // Don't fail the upload for local storage issues
                err
            })
            .ok();

            Ok::<(), UploadError>(())
        }
        .await;

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
