use autonomi::self_encryption::EncryptionStream;
use autonomi::XorName;

pub const MAX_CHUNKS_PER_BATCH: usize = 64;

pub(crate) async fn content_addresses_from_encryption_stream(
    encryption_stream: &mut EncryptionStream,
) -> Vec<(XorName, usize)> {
    let mut content_addresses = vec![];

    while let Some(next_batch) = encryption_stream.next_batch(MAX_CHUNKS_PER_BATCH) {
        content_addresses.extend(
            next_batch.into_iter()
                .map(|chunk| (*chunk.name(), chunk.size()))
        );
    }

    content_addresses
}
