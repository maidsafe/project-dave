use std::path::{PathBuf};
use autonomi::{Bytes, Chunk};
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use crate::ant::payments::PaymentOrderManager;

#[derive(Deserialize)]
pub struct File {
    name: String,
    path: PathBuf
}

pub struct FileEncrypted {
    name: String,
    datamap: Chunk,
    chunks: Vec<Chunk>
}

pub async fn read_file_to_bytes(file_path: PathBuf) -> Bytes {
    Bytes::from(tokio::fs::read(file_path).await.expect("Failed to read file"))
}

pub async fn upload_files(app: AppHandle, files: Vec<File>, archive_name: &str, payment_orders: State<'_, PaymentOrderManager>) {
    let mut encrypted_files = vec![];

    for file in files {
        let bytes: Bytes = read_file_to_bytes(file.path).await;

        let (datamap, chunks) = autonomi::self_encryption::encrypt(bytes).unwrap();

        encrypted_files.push(FileEncrypted {
            name: file.name,
            datamap,
            chunks,
        })
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