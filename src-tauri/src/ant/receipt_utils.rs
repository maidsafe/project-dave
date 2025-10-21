use autonomi::client::payment::Receipt;
use autonomi::XorName;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ReceiptValidation {
    pub missing_chunks: Vec<Vec<u8>>,
    pub is_complete: bool,
}

pub fn validate_receipt_coverage_with_content_addresses(
    receipt: &Receipt,
    content_addresses: &[XorName],
) -> ReceiptValidation {
    // Extract chunk addresses covered by the receipt
    let covered_chunks = extract_covered_chunks(receipt);

    println!(
        ">>> Receipt validation: receipt covers {} chunks, quote requires {} chunks",
        covered_chunks.len(),
        content_addresses.len()
    );

    // Check which chunks from the quote are missing in the receipt
    let mut missing_chunks = Vec::new();

    for xorname in content_addresses {
        let chunk_name = xorname.to_vec();
        if !covered_chunks.contains(&chunk_name) {
            missing_chunks.push(chunk_name);
        }
    }

    if !missing_chunks.is_empty() {
        println!(
            ">>> Receipt validation: {} chunks are missing from the cached receipt",
            missing_chunks.len()
        );
    } else {
        println!(">>> Receipt validation: All chunks from quote are covered by cached receipt");
    }

    ReceiptValidation {
        is_complete: missing_chunks.is_empty(),
        missing_chunks,
    }
}

pub fn merge_receipts(receipts: Vec<Receipt>) -> Receipt {
    if receipts.is_empty() {
        panic!("Cannot merge empty receipt list");
    }

    if receipts.len() == 1 {
        return receipts.into_iter().next().unwrap();
    }

    println!(">>> Merging {} receipts", receipts.len());

    // Merge all receipts (HashMap) into one
    let mut merged_receipt = Receipt::new();
    let mut total_chunks_merged = 0;

    for (idx, receipt) in receipts.into_iter().enumerate() {
        let chunks_in_receipt = receipt.len();
        println!(
            ">>> Receipt {}: contains {} chunks",
            idx + 1,
            chunks_in_receipt
        );

        // Merge all entries from this receipt
        for (xorname, payment_data) in receipt {
            merged_receipt.insert(xorname, payment_data);
        }
        total_chunks_merged += chunks_in_receipt;
    }

    println!(
        ">>> Merged receipt contains {} unique chunks (from {} total chunks)",
        merged_receipt.len(),
        total_chunks_merged
    );

    merged_receipt
}

fn extract_covered_chunks(receipt: &Receipt) -> HashSet<Vec<u8>> {
    let mut covered = HashSet::new();

    // Receipt is a HashMap<XorName, (ClientProofOfPayment, AttoTokens)>
    for xorname in receipt.keys() {
        covered.insert(xorname.0.to_vec());
    }

    covered
}
