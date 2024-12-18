use crate::ant::client::client;
use crate::ant::payments::{OrderMessage, PaymentOrderManager, IDLE_PAYMENT_TIMEOUT_SECS};
use autonomi::client::data::DataMapChunk;
use autonomi::client::files::archive::Metadata;
use autonomi::client::quote::StoreQuote;
use autonomi::{Bytes, Chunk};
use rand::Rng;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::time::timeout;

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

pub async fn upload_private_files_to_vault(
    app: AppHandle,
    files: Vec<File>,
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

    let mut aggregated_store_quote = StoreQuote(Default::default());

    let mut private_archive = autonomi::PrivateArchive::new();

    for file in &encrypted_files {
        private_archive.add_file(
            PathBuf::from(&file.name),
            DataMapChunk::from(file.datamap.clone()),
            file.metadata.clone(),
        );

        let chunk_addresses: Vec<_> = file
            .chunks
            .iter()
            .map(|chunk| *chunk.address.xorname())
            .collect();

        let store_quote = client
            .get_store_quotes(chunk_addresses.into_iter())
            .await
            .unwrap();

        for (xor_name, quotes) in store_quote.0 {
            aggregated_store_quote.0.entry(xor_name).or_insert(quotes);
        }
    }

    let (private_archive_datamap, private_archive_chunks) = autonomi::self_encryption::encrypt(
        private_archive
            .into_bytes()
            .expect("Could not produce bytes from private archive"),
    )
    .unwrap();

    let private_archive_chunk_addresses: Vec<_> = private_archive_chunks
        .iter()
        .map(|chunk| *chunk.address.xorname())
        .collect();

    let private_archive_store_quote = client
        .get_store_quotes(private_archive_chunk_addresses.into_iter())
        .await
        .unwrap();

    for (xor_name, quotes) in private_archive_store_quote.0 {
        aggregated_store_quote.0.entry(xor_name).or_insert(quotes);
    }

    let (order, mut confirmation_receiver) = payment_orders.create_order(vec![]);

    // let the frontend know that a payment has to be made
    app.emit("payment-order", order.to_json()).unwrap();

    let order_successful = tokio::spawn(async move {
        loop {
            let result = timeout(
                Duration::from_secs(IDLE_PAYMENT_TIMEOUT_SECS),
                confirmation_receiver.recv(),
            )
            .await;

            match result {
                Ok(Some(order_message)) => match order_message {
                    OrderMessage::KeepAlive => {
                        continue;
                    }
                    OrderMessage::Completed => {
                        return true;
                    }
                    OrderMessage::Cancelled => {
                        return false;
                    }
                },
                _ => {
                    return false;
                }
            };
        }
    })
    .await;

    // upload chunks and archive

    todo!()
}

pub async fn payment_test(app: AppHandle, payment_orders: State<'_, PaymentOrderManager>) {
    println!("Running test!");

    println!("Connecting to client..");

    let client = client().await;

    println!("Client connected!");

    let bytes: Vec<u8> = (0..1024).map(|_| rand::thread_rng().gen()).collect();

    println!("Encrypting bytes..");

    let (_datamap, chunks) = autonomi::self_encryption::encrypt(Bytes::from(bytes)).unwrap();

    println!("Got chunks!");

    let chunk_addresses: Vec<_> = chunks
        .iter()
        .map(|chunk| *chunk.address.xorname())
        .collect();

    println!("Getting quotes..");

    let store_quote = client
        .get_store_quotes(chunk_addresses.into_iter())
        .await
        .unwrap();

    println!("Got quotes!");

    let (payment_order, mut confirmation_receiver) =
        payment_orders.create_order(store_quote.payments());

    // let the frontend know that a payment has to be made
    app.emit("payment-order", payment_order.to_json()).unwrap();

    let order_successful = tokio::spawn(async move {
        loop {
            let result = timeout(
                Duration::from_secs(IDLE_PAYMENT_TIMEOUT_SECS),
                confirmation_receiver.recv(),
            )
            .await;

            match result {
                Ok(Some(order_message)) => match order_message {
                    OrderMessage::KeepAlive => {
                        continue;
                    }
                    OrderMessage::Completed => {
                        return true;
                    }
                    OrderMessage::Cancelled => {
                        return false;
                    }
                },
                _ => {
                    return false;
                }
            };
        }
    })
    .await
    .unwrap();

    println!("Order paid: {order_successful}");
}
