use crate::ant::client::SharedClient;
use crate::ant::payments::{OrderMessage, PaymentOrderManager, IDLE_PAYMENT_TIMEOUT_SECS};
use autonomi::chunk::DataMapChunk;
use autonomi::client::quote::{DataTypes, StoreQuote};
use autonomi::client::vault::key::vault_key_from_signature_hex;
use autonomi::client::vault::{app_name_to_vault_content_type, UserData, VaultSecretKey};
use autonomi::client::GetError;
use autonomi::data::DataAddress;
use autonomi::files::{Metadata, PrivateArchive};
use autonomi::vault::user_data::UserDataVaultError;
use autonomi::{Amount, Bytes, Chunk, Scratchpad, ScratchpadAddress};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use thiserror::Error as ThisError;
use tokio::fs;
use tokio::time::timeout;

#[derive(Deserialize)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum UploadProgress {
    Started {
        total_files: usize,
        total_size: u64,
    },
    Processing {
        current_file: String,
        files_processed: usize,
        total_files: usize,
        bytes_processed: u64,
        total_bytes: u64,
    },
    Encrypting {
        current_file: String,
        files_processed: usize,
        total_files: usize,
    },
    RequestingPayment {
        files_processed: usize,
        total_files: usize,
    },
    Uploading {
        chunks_uploaded: usize,
        total_chunks: usize,
        bytes_uploaded: u64,
        total_bytes: u64,
    },
    Completed {
        total_files: usize,
        total_bytes: u64,
    },
    Failed {
        error: String,
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
    let dir_name = dir_path.file_name().unwrap_or_default();

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
                    // Ensure the root folder name is included in the path
                    let mut relative_path = PathBuf::new();
                    if let Some(name) = dir_name.to_str() {
                        relative_path.push(name);
                    }

                    // Add the rest of the path
                    if rel_path != dir_name {
                        relative_path.push(rel_path);
                    }

                    files.push((relative_path, path));
                } else {
                    // Fallback if strip_prefix fails
                    files.push((path.clone(), path));
                }
            }
        }
    }

    Ok(files)
}

pub async fn upload_private_files_to_vault(
    app: AppHandle,
    files: Vec<File>,
    archive_name: String,
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), UploadError> {
    let client = shared_client.get_client().await?;

    // Calculate total size and emit start event
    let total_size = calculate_total_size(&files).await?;
    let total_files = files.len();

    app.emit(
        "upload-progress",
        UploadProgress::Started {
            total_files,
            total_size,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    let mut aggregated_chunks: Vec<Chunk> = vec![];
    let mut aggregated_store_quote = StoreQuote(Default::default());
    let mut private_archive = PrivateArchive::new();
    let mut files_processed = 0;
    let mut bytes_processed = 0u64;

    for file in files {
        // Check if the path is a directory
        let path_metadata = fs::metadata(&file.path)
            .await
            .map_err(|_| UploadError::Read(file.path.clone()))?;

        if path_metadata.is_dir() {
            // Handle directory: collect all files with their relative paths
            let collected_files = collect_files_from_directory(file.path.clone()).await?;

            for (relative_path, absolute_path) in collected_files {
                // Emit processing progress
                app.emit(
                    "upload-progress",
                    UploadProgress::Processing {
                        current_file: relative_path.to_string_lossy().to_string(),
                        files_processed,
                        total_files,
                        bytes_processed,
                        total_bytes: total_size,
                    },
                )
                .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

                let bytes: Bytes = read_file_to_bytes(absolute_path.clone()).await?;
                let file_size = bytes.len() as u64;

                // Emit encrypting progress
                app.emit(
                    "upload-progress",
                    UploadProgress::Encrypting {
                        current_file: relative_path.to_string_lossy().to_string(),
                        files_processed,
                        total_files,
                    },
                )
                .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

                let metadata = Metadata::new_with_size(file_size);
                let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes)
                    .map_err(|err| UploadError::Encryption(err.to_string()))?;

                bytes_processed += file_size;

                // Add file to archive with its relative path
                private_archive.add_file(
                    relative_path,
                    DataMapChunk::from(datamap.clone()),
                    metadata.clone(),
                );

                let chunks_iter = chunks
                    .iter()
                    .map(|chunk| (*chunk.address.xorname(), chunk.value.len()));

                let store_quote = client
                    .get_store_quotes(DataTypes::Chunk, chunks_iter)
                    .await
                    .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

                for (xor_name, quotes) in store_quote.0 {
                    aggregated_store_quote.0.entry(xor_name).or_insert(quotes);
                }

                for chunk in chunks {
                    aggregated_chunks.push(chunk);
                }
            }
        } else {
            // Handle single file
            // Emit processing progress
            app.emit(
                "upload-progress",
                UploadProgress::Processing {
                    current_file: file.name.clone(),
                    files_processed,
                    total_files,
                    bytes_processed,
                    total_bytes: total_size,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            let bytes: Bytes = read_file_to_bytes(file.path.clone()).await?;
            let file_size = bytes.len() as u64;

            // Emit encrypting progress
            app.emit(
                "upload-progress",
                UploadProgress::Encrypting {
                    current_file: file.name.clone(),
                    files_processed,
                    total_files,
                },
            )
            .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

            let metadata = Metadata::new_with_size(file_size);
            let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes)
                .map_err(|err| UploadError::Encryption(err.to_string()))?;

            bytes_processed += file_size;

            private_archive.add_file(
                PathBuf::from(&file.name),
                DataMapChunk::from(datamap.clone()),
                metadata.clone(),
            );

            let chunks_iter = chunks
                .iter()
                .map(|chunk| (*chunk.address.xorname(), chunk.value.len()));

            let store_quote = client
                .get_store_quotes(DataTypes::Chunk, chunks_iter)
                .await
                .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

            for (xor_name, quotes) in store_quote.0 {
                aggregated_store_quote.0.entry(xor_name).or_insert(quotes);
            }

            for chunk in chunks {
                aggregated_chunks.push(chunk);
            }
        }

        files_processed += 1;
    }

    let (private_archive_datamap, private_archive_chunks) = autonomi::self_encryption::encrypt(
        private_archive
            .to_bytes()
            .map_err(|err| UploadError::Encryption(err.to_string()))?,
    )
    .map_err(|err| UploadError::Encryption(err.to_string()))?;

    let private_archive_chunk_iter = private_archive_chunks
        .iter()
        .map(|chunk| (*chunk.address.xorname(), chunk.value.len()));

    let private_archive_store_quote = client
        .get_store_quotes(DataTypes::Chunk, private_archive_chunk_iter)
        .await
        .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

    for (xor_name, quotes) in private_archive_store_quote.0 {
        aggregated_store_quote.0.entry(xor_name).or_insert(quotes);
    }

    for chunk in private_archive_chunks {
        aggregated_chunks.push(chunk);
    }

    let mut user_data = client
        .get_user_data_from_vault(secret_key)
        .await
        .unwrap_or(UserData::new());

    // Add the archive with a name
    user_data
        .private_file_archives
        .insert(DataMapChunk::from(private_archive_datamap), archive_name);

    let scratchpad_addr = ScratchpadAddress::new(secret_key.public_key());
    let scratchpad_exists = client
        .scratchpad_check_existence(&scratchpad_addr)
        .await
        .map_err(|err| UploadError::Scratchpad(format!("{err}")))?;
    let content_type = app_name_to_vault_content_type("UserData");
    let scratchpad = if scratchpad_exists {
        client
            .scratchpad_get(&scratchpad_addr)
            .await
            .map_err(|err| UploadError::Scratchpad(format!("{err}")))?
    } else {
        Scratchpad::new(secret_key, content_type, &Bytes::new(), 0)
    };

    let scratch_pad_store_quote = client
        .get_store_quotes(
            DataTypes::Scratchpad,
            std::iter::once((scratchpad.address().xorname(), scratchpad.size())),
        )
        .await
        .map_err(|err| UploadError::StoreQuote(err.to_string()))?;

    for (xor_name, quotes) in scratch_pad_store_quote.0 {
        aggregated_store_quote.0.entry(xor_name).or_insert(quotes);
    }

    let payments: Vec<_> = aggregated_store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    // Calculate total cost
    let total_cost: Amount = payments.iter().map(|(_, _, amount)| *amount).sum();
    let has_payments = !payments.is_empty();

    // Emit payment request progress
    app.emit(
        "upload-progress",
        UploadProgress::RequestingPayment {
            files_processed,
            total_files,
        },
    )
    .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    let (order, mut confirmation_receiver) = payment_orders.create_order(payments.clone()).await;

    // Emit simplified payment request
    let payment_request = serde_json::json!({
        "order_id": order.id,
        "total_cost_nano": total_cost.to_string(),
        "total_cost_formatted": format!("{} ATTO", total_cost),
        "payment_required": has_payments,
        "payments": payments
    });

    tracing::debug!(
        "Emitting payment-request event with data: {:?}",
        payment_request
    );
    app.emit("payment-request", payment_request)
        .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    let secret_key = secret_key.clone();
    let app_clone = app.clone();
    let total_chunks = aggregated_chunks.len();

    tokio::spawn(async move {
        let order_successful = loop {
            match timeout(
                Duration::from_secs(IDLE_PAYMENT_TIMEOUT_SECS),
                confirmation_receiver.recv(),
            )
            .await
            {
                Ok(Some(OrderMessage::KeepAlive)) => continue,
                Ok(Some(OrderMessage::Completed)) => break true,
                _ => break false,
            }
        };

        tracing::debug!("Order paid: {order_successful}");

        if order_successful {
            let receipt =
                autonomi::client::payment::receipt_from_store_quotes(aggregated_store_quote);

            tracing::debug!("Uploading chunks..");

            // Emit uploading progress
            tracing::debug!(
                "Emitting initial Uploading progress: 0/{} chunks",
                total_chunks
            );
            let _ = app_clone.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    chunks_uploaded: 0,
                    total_chunks,
                    bytes_uploaded: 0,
                    total_bytes: total_size,
                },
            );

            let failed_uploads = client
                .chunk_batch_upload(aggregated_chunks.iter().collect(), &receipt)
                .await;

            if let Err(err) = failed_uploads {
                tracing::error!("Upload chunks errored: {err}");
                let _ = app_clone.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        error: format!("Upload failed: {}", err),
                    },
                );
                return;
            }

            // Emit final progress update showing all chunks uploaded
            tracing::debug!(
                "Emitting final Uploading progress: {}/{} chunks",
                total_chunks,
                total_chunks
            );
            let _ = app_clone.emit(
                "upload-progress",
                UploadProgress::Uploading {
                    chunks_uploaded: total_chunks,
                    total_chunks,
                    bytes_uploaded: total_size,
                    total_bytes: total_size,
                },
            );

            let result = client
                .put_user_data_to_vault(&secret_key, receipt.into(), user_data)
                .await;

            tracing::debug!("Update vault result: {:?}", result);

            if result.is_ok() {
                // Emit completion
                let _ = app_clone.emit(
                    "upload-progress",
                    UploadProgress::Completed {
                        total_files,
                        total_bytes: total_size,
                    },
                );
            } else {
                let _ = app_clone.emit(
                    "upload-progress",
                    UploadProgress::Failed {
                        error: "Failed to update vault".to_string(),
                    },
                );
            }
        } else {
            let _ = app_clone.emit(
                "upload-progress",
                UploadProgress::Failed {
                    error: "Payment was not completed".to_string(),
                },
            );
        }
    });

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileFromVault {
    path: String,
    metadata: Metadata,
    file_access: PublicOrPrivateFile,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FailedArchive {
    pub name: String,
    pub is_private: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStructure {
    pub archives: Vec<ArchiveInfo>,
    pub failed_archives: Vec<FailedArchive>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveInfo {
    pub name: String,
    pub is_private: bool,
    pub files: Vec<FileMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub metadata: Metadata,
    pub file_type: FileType,
    pub is_loaded: bool,
    pub archive_name: String,
    pub access_data: Option<PublicOrPrivateFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FileType {
    Public,
    Private,
}

#[derive(Debug, Serialize, Deserialize)]
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
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

        if let Ok(archive) = client.archive_get(&data_map).await {
            let mut files: Vec<FileMetadata> = vec![];

            for (filepath, (data_map, metadata)) in archive.map() {
                println!("archive name: {:?}", archive_name);
                println!("filepath: {:?}", filepath);

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
                name: archive_name,
                is_private: true,
                files,
            });
        } else {
            tracing::error!("Failed to get private archive: {}", archive_name);
            failed_archives.push(FailedArchive {
                name: archive_name,
                is_private: true,
            });
        }
    }

    // Process public archives
    for (archive_addr, name) in user_data.file_archives {
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

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
                name: archive_name,
                is_private: false,
                files,
            });
        } else {
            tracing::error!("Failed to get public archive: {}", archive_name);
            failed_archives.push(FailedArchive {
                name: archive_name,
                is_private: false,
            });
        }
    }

    Ok(VaultStructure {
        archives,
        failed_archives,
    })
}

pub async fn get_files_from_vault(
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client
        .get_user_data_from_vault(secret_key)
        .await
        .inspect_err(|_| {
            tracing::error!("Failed to get user data from vault");
        })?;

    let mut files: Vec<FileFromVault> = vec![];

    for (data_map, name) in user_data.private_file_archives {
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

        // Get private archive
        let archive = client.archive_get(&data_map).await.inspect_err(|_| {
            tracing::error!("Failed to get private archive");
        })?;

        for (filepath, (data_map, metadata)) in archive.map() {
            let filepath = format!("{archive_name}/{}", filepath.display());

            let file = FileFromVault {
                path: filepath,
                metadata: metadata.clone(),
                file_access: PublicOrPrivateFile::Private(data_map.clone()),
            };
            files.push(file);
        }
    }

    for (archive_addr, name) in user_data.file_archives {
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

        // Get public archive
        let archive = client
            .archive_get_public(&archive_addr)
            .await
            .inspect_err(|_| {
                tracing::error!("Failed to get public archive");
            })?;

        for (filepath, (data_addr, metadata)) in archive.map() {
            let filepath = format!("{archive_name}/{}", filepath.display());

            let file = FileFromVault {
                path: filepath,
                metadata: metadata.clone(),
                file_access: PublicOrPrivateFile::Public(*data_addr),
            };
            files.push(file);
        }
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

    // Try to find the file in private archives first
    for (data_map, archive_name) in user_data.private_file_archives.iter() {
        // Check if the file path starts with this archive name
        if file_path.starts_with(&format!("{}/", archive_name)) {
            let archive = client.archive_get(data_map).await?;

            // Look for the file in this archive
            for (filepath, (file_data_map, metadata)) in archive.map() {
                let full_path = format!("{}/{}", archive_name, filepath.display());
                if full_path == file_path {
                    return Ok(FileFromVault {
                        path: file_path.to_string(),
                        metadata: metadata.clone(),
                        file_access: PublicOrPrivateFile::Private(file_data_map.clone()),
                    });
                }
            }
        }
    }

    // Try to find the file in public archives
    for (addr, archive_name) in user_data.file_archives.iter() {
        // Check if the file path starts with this archive name
        if file_path.starts_with(&format!("{}/", archive_name)) {
            let archive = client.archive_get_public(addr).await?;

            // Look for the file in this archive
            for (filepath, (file_addr, metadata)) in archive.map() {
                let full_path = format!("{}/{}", archive_name, filepath.display());
                if full_path == file_path {
                    return Ok(FileFromVault {
                        path: file_path.to_string(),
                        metadata: metadata.clone(),
                        file_access: PublicOrPrivateFile::Public(*file_addr),
                    });
                }
            }
        }
    }

    Err(VaultError::FileNotFound)
}
