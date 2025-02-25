use crate::ant::client::SharedClient;
use crate::ant::payments::{OrderMessage, PaymentOrderManager, IDLE_PAYMENT_TIMEOUT_SECS};
use autonomi::chunk::DataMapChunk;
use autonomi::client::data::DataAddr;
use autonomi::client::quote::{DataTypes, StoreQuote};
use autonomi::client::vault::{app_name_to_vault_content_type, UserData, VaultSecretKey};
use autonomi::client::GetError;
use autonomi::files::{Metadata, PrivateArchive};
use autonomi::vault::user_data::UserDataVaultError;
use autonomi::{Amount, Bytes, Chunk, Scratchpad, ScratchpadAddress};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use thiserror::Error as ThisError;
use tokio::time::timeout;

#[derive(Deserialize)]
pub struct File {
    name: String,
    path: PathBuf,
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

pub async fn upload_private_files_to_vault(
    app: AppHandle,
    files: Vec<File>,
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
    payment_orders: State<'_, PaymentOrderManager>,
) -> Result<(), UploadError> {
    let client = shared_client.get_client().await?;

    let mut aggregated_chunks: Vec<Chunk> = vec![];
    let mut aggregated_store_quote = StoreQuote(Default::default());
    let mut private_archive = PrivateArchive::new();

    for file in files {
        let bytes: Bytes = read_file_to_bytes(file.path).await?;
        let metadata = Metadata::new_with_size(bytes.len() as u64);
        let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes)
            .map_err(|err| UploadError::Encryption(err.to_string()))?;

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

    let _ = user_data.add_private_file_archive(DataMapChunk::from(private_archive_datamap));

    let scratchpad_addr = ScratchpadAddress::new(secret_key.public_key());
    let scratchpad_exists = client
        .scratchpad_check_existance(&scratchpad_addr)
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

    let payments = aggregated_store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    let (order, mut confirmation_receiver) = payment_orders.create_order(payments).await;

    app.emit("payment-order", order.to_json())
        .map_err(|err| UploadError::EmitEvent(err.to_string()))?;

    let secret_key = secret_key.clone();

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

            let failed_uploads = client
                .upload_chunks_with_retries(aggregated_chunks.iter().collect(), &receipt)
                .await;

            tracing::debug!("Failed uploads: {}", failed_uploads.len());

            let result = client
                .put_user_data_to_vault(&secret_key, receipt.into(), user_data)
                .await;

            tracing::debug!("Update vault result: {:?}", result);
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
pub enum PublicOrPrivateFile {
    Public(DataAddr),
    Private(DataMapChunk),
}

pub async fn get_files_from_vault(
    secret_key: &VaultSecretKey,
    shared_client: State<'_, SharedClient>,
) -> Result<Vec<FileFromVault>, VaultError> {
    let client = shared_client.get_client().await?;

    // Fetch user data
    let user_data = client.get_user_data_from_vault(secret_key).await?;

    let mut files: Vec<FileFromVault> = vec![];

    for (data_map, name) in user_data.private_file_archives {
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

        // Get private archive
        let archive = client.archive_get(&data_map).await?;

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
        let archive = client.archive_get_public(&archive_addr).await?;

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
    client.file_download(data_map, to_dest).await?;
    Ok(())
}

pub async fn download_public_file(
    addr: &DataAddr,
    to_dest: PathBuf,
    shared_client: State<'_, SharedClient>,
) -> Result<(), DownloadError> {
    let client = shared_client.get_client().await?;
    client.file_download_public(addr, to_dest).await?;
    Ok(())
}
