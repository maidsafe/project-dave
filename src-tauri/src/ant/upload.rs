use crate::ant::stream::MAX_CHUNKS_PER_BATCH;
use autonomi::client::payment::Receipt;
use autonomi::self_encryption::EncryptionStream;
use autonomi::Client;

#[derive(Debug)]
pub(crate) enum UploadError {
    Put(String),
}

pub(crate) async fn batch_upload_encryption_stream(
    client: &Client,
    receipt: &Receipt,
    encryption_stream: &mut EncryptionStream,
) -> Result<(), UploadError> {
    while let Some(next_batch) = encryption_stream.next_batch(MAX_CHUNKS_PER_BATCH) {
        client
            .chunk_batch_upload(next_batch.iter().collect(), receipt)
            .await
            .map_err(|err| UploadError::Put(err.to_string()))?;
    }

    Ok(())
}
