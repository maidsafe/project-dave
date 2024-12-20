use crate::ant::client::client;
use crate::ant::payments::{OrderMessage, PaymentOrderManager, IDLE_PAYMENT_TIMEOUT_SECS};
use autonomi::client::data::{DataAddr, DataMapChunk};
use autonomi::client::files::archive::Metadata;
use autonomi::client::files::fs::DownloadError;
use autonomi::client::quote::StoreQuote;
use autonomi::client::vault::user_data::UserDataVaultGetError;
use autonomi::client::vault::VaultSecretKey;
use autonomi::{Amount, Bytes, Chunk};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::MessageDialogButtons;
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
            .to_bytes()
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

    let payments = aggregated_store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    let (order, mut confirmation_receiver) = payment_orders.create_order(payments).await;

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

    let payments = store_quote
        .payments()
        .into_iter()
        .filter(|(_, _, amount)| *amount > Amount::ZERO)
        .collect();

    let (payment_order, mut confirmation_receiver) = payment_orders.create_order(payments).await;

    // let the frontend know that a payment has to be made
    app.emit("payment-order", payment_order).unwrap();

    println!("Chunks: {chunks:?}");

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

        println!("Order paid: {order_successful}");

        if order_successful {
            let receipt = autonomi::client::payment::receipt_from_store_quotes(store_quote);

            println!("Uploading chunks..");

            for chunk in &chunks {
                let proof_of_payment = receipt.get(chunk.address.xorname());
                println!("Chunk: {chunk:?}, Proof of payment: {proof_of_payment:?}");
            }

            let result = client
                .upload_chunks_with_retries(chunks.iter().collect(), &receipt)
                .await;

            println!("Upload result: {result:?}");
        }
    });
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
) -> Result<Vec<FileFromVault>, UserDataVaultGetError> {
    let client = client().await;

    // Fetch user data
    let user_data = client.get_user_data_from_vault(secret_key).await?;

    let mut files: Vec<FileFromVault> = vec![];

    for (data_map, name) in user_data.private_file_archives {
        let archive_name = name.replace(",", "-").replace("/", "-").replace(" ", "");

        // Get private archive
        let archive = client.archive_get(data_map).await?;

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
        let archive = client.archive_get_public(archive_addr).await?;

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
    data_map: DataMapChunk,
    to_dest: PathBuf,
) -> Result<(), DownloadError> {
    let client = client().await;
    client.file_download(data_map, to_dest).await?;

    Ok(())
}

pub async fn download_public_file(addr: DataAddr, to_dest: PathBuf) -> Result<(), DownloadError> {
    let client = client().await;
    client.file_download_public(addr, to_dest).await?;

    Ok(())
}
