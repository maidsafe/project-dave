use autonomi::self_encryption::EncryptionStream;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Debug)]
pub(crate) enum EncryptionError {
    Encryption(String),
    IO(String),
}

pub(crate) async fn encrypt_file_or_folder(
    path: PathBuf,
    is_public: bool,
) -> Result<Vec<EncryptionStream>, EncryptionError> {
    let encryption_results = autonomi::self_encryption::encrypt_directory_files(path, is_public)
        .await
        .map_err(|err| EncryptionError::IO(err.to_string()))?;

    let mut encryption_streams = vec![];

    for encryption_result in encryption_results {
        match encryption_result {
            Ok(file_chunk_iterator) => {
                let file_path = file_chunk_iterator.file_path.clone();
                info!("Successfully encrypted file: {file_path:?}");

                encryption_streams.push(file_chunk_iterator);
            }
            Err(err_msg) => {
                error!("Error during file encryption: {err_msg}");
                return Err(EncryptionError::Encryption(err_msg));
            }
        }
    }

    Ok(encryption_streams)
}
