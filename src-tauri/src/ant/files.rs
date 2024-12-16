use autonomi::{Bytes, Chunk};
use tauri::{AppHandle, Emitter, State};
use crate::ant::payments::PaymentOrderManager;

pub struct File {
    name: String,
    data: Bytes
}

pub struct FileEncrypted {
    name: String,
    datamap: Chunk,
    chunks: Vec<Chunk>
}

pub async fn upload_files(app: AppHandle, files: Vec<File>, archive_name: &str, payment_orders: State<PaymentOrderManager>) {
    let mut encrypted_files = vec![];

    for file in files {
        let (datamap, chunks) = autonomi::self_encryption::encrypt(file.data).unwrap();

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