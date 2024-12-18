use crate::ant::client::client;
use crate::ant::payments::PaymentOrderManager;
use autonomi::client::data::DataMapChunk;
use autonomi::client::files::archive::Metadata;
use autonomi::client::vault::user_data::UserDataVaultGetError;
use autonomi::client::vault::VaultSecretKey;
use autonomi::{Bytes, Chunk};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[derive(Deserialize)]
pub struct File {
    name: String,
    path: PathBuf,
}

pub struct FileEncrypted {
    name: String,
    metadata: Metadata,
    datamap: Chunk,
    chunks: Vec<Chunk>,
}

pub async fn read_file_to_bytes(file_path: PathBuf) -> Bytes {
    Bytes::from(
        tokio::fs::read(file_path)
            .await
            .expect("Failed to read file"),
    )
}

pub async fn upload_files(
    app: AppHandle,
    files: Vec<File>,
    archive_name: &str,
    payment_orders: State<'_, PaymentOrderManager>,
) {
    let client = client().await;

    let mut encrypted_files = vec![];

    for file in files {
        let bytes: Bytes = read_file_to_bytes(file.path).await;
        let metadata = Metadata::new_with_size(bytes.len() as u64);
        let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes).unwrap();

        encrypted_files.push(FileEncrypted {
            name: file.name,
            metadata,
            datamap,
            chunks,
        })
    }

    let mut private_archive = autonomi::PrivateArchive::new();

    // let mut store_quotes = vec![];

    for file in &encrypted_files {
        private_archive.add_file(
            PathBuf::from(&file.name),
            DataMapChunk::from(file.datamap.clone()),
            file.metadata.clone(),
        );

        // let chunk_addresses = file.chunks.iter().map(|chunk| chunk.address).collect();

        // client.get_store_quotes()
    }

    // build archive
    // fetch quotes

    let (order, confirmation_receiver) = payment_orders.create_order(vec![]);

    // let the frontend know that a payment has to be made
    app.emit("payment-order", order.to_json()).unwrap();

    // request payment of quotes to frontend
    // upload chunks and archive
    todo!()
}

#[derive(Deserialize)]
pub struct PrivateFileFromVault {
    paths: PrivateFileFromVaultPath,
    size: u64,
    date_created: u64,
    date_modified: u64,
    date_uploaded: u64,
    private_data_access: DataMapChunk,
}

#[derive(Deserialize)]
pub struct PrivateFileFromVaultPath {
    local: String,
}

pub async fn get_private_files_from_vault(
    secret_key: &VaultSecretKey,
) -> Result<Vec<PrivateFileFromVault>, UserDataVaultGetError> {
    let client = client().await;

    // Fetch user data
    let user_data = client.get_user_data_from_vault(secret_key).await?;

    let mut files: Vec<PrivateFileFromVault> = vec![];

    for (data_map, name) in user_data.private_file_archives {
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

        // Get private archive
        let archive = client.archive_get(data_map).await?;

        for (filename, (data_map, metadata)) in archive.map() {
            let filepath = format!("{archive_name}/{}", filename.display());

            let file = PrivateFileFromVault {
                paths: PrivateFileFromVaultPath { local: filepath },
                size: metadata.size,
                date_created: metadata.created,
                date_modified: metadata.modified,
                date_uploaded: metadata.uploaded,
                private_data_access: data_map.clone(),
            };
            files.push(file);
        }
    }

    Ok(files)
}

#[derive(Deserialize)]
pub struct PublicFileFromVault {
    paths: PublicFileFromVaultPath,
    size: u64,
    date_created: u64,
    date_modified: u64,
    date_uploaded: u64,
    data_addr: [u8; 32],
}

#[derive(Deserialize)]
pub struct PublicFileFromVaultPath {
    local: String,
}

pub async fn get_public_files_from_vault(
    secret_key: &VaultSecretKey,
) -> Result<Vec<PublicFileFromVault>, UserDataVaultGetError> {
    let client = client().await;

    // Fetch user data
    let user_data = client.get_user_data_from_vault(secret_key).await?;

    let mut files: Vec<PublicFileFromVault> = vec![];

    for (archive_addr, name) in user_data.file_archives {
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

        // Get public archive
        let archive = client.archive_get_public(archive_addr).await?;

        for (filename, (data_addr, metadata)) in archive.map() {
            let filepath = format!("{archive_name}/{}", filename.display());

            let file = PublicFileFromVault {
                paths: PublicFileFromVaultPath { local: filepath },
                size: metadata.size,
                date_created: metadata.created,
                date_modified: metadata.modified,
                date_uploaded: metadata.uploaded,
                data_addr: data_addr.0,
            };
            files.push(file);
        }
    }

    Ok(files)
}
