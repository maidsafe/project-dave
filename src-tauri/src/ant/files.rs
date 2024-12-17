use crate::ant::client::client;
use crate::ant::payments::PaymentOrderManager;
use autonomi::client::data::DataMapChunk;
use autonomi::client::files::archive::Metadata;
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
