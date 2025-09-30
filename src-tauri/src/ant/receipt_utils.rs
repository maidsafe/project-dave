use autonomi::client::payment::Receipt;
use autonomi::chunk::DataMapChunk;
use autonomi::Chunk;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ReceiptValidation {
    pub covered_chunks: HashSet<Vec<u8>>,
    pub missing_chunks: Vec<Vec<u8>>,
    pub is_complete: bool,
}

pub fn validate_receipt_coverage(
    receipt: &Receipt,
    required_chunks: &[Chunk],
) -> ReceiptValidation {
    validate_receipt_coverage_with_datamaps(receipt, required_chunks, &[])
}

pub fn validate_receipt_coverage_with_datamaps(
    receipt: &Receipt,
    required_chunks: &[Chunk],
    required_datamaps: &[DataMapChunk],
) -> ReceiptValidation {
    // Extract chunk addresses covered by the receipt
    let covered_chunks = extract_covered_chunks(receipt);
    
    let total_required = required_chunks.len() + required_datamaps.len();
    println!(">>> Receipt validation: receipt covers {} chunks, {} chunks + {} datamaps = {} total required", 
        covered_chunks.len(), required_chunks.len(), required_datamaps.len(), total_required);
    
    // Check which chunks are missing
    let mut missing_chunks = Vec::new();
    
    // Check data chunks
    for chunk in required_chunks {
        let chunk_name = chunk.name().to_vec();
        if !covered_chunks.contains(&chunk_name) {
            missing_chunks.push(chunk_name);
        }
    }
    
    // Check datamap chunks  
    for datamap in required_datamaps {
        let datamap_name = datamap.0.name().0.to_vec();
        if !covered_chunks.contains(&datamap_name) {
            missing_chunks.push(datamap_name);
        }
    }
    
    if !missing_chunks.is_empty() {
        println!(">>> Receipt validation: {} chunks/datamaps are missing from the cached receipt", 
            missing_chunks.len());
    } else {
        println!(">>> Receipt validation: All chunks and datamaps are covered by cached receipt");
    }
    
    ReceiptValidation {
        covered_chunks,
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
        println!(">>> Receipt {}: contains {} chunks", idx + 1, chunks_in_receipt);
        
        // Merge all entries from this receipt
        for (xorname, payment_data) in receipt {
            merged_receipt.insert(xorname, payment_data);
        }
        total_chunks_merged += chunks_in_receipt;
    }
    
    println!(">>> Merged receipt contains {} unique chunks (from {} total chunks)", 
        merged_receipt.len(), total_chunks_merged);
    
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_receipt_validation() {
        // Test implementation would go here
    }
    
    #[test]
    fn test_receipt_merging() {
        // Test implementation would go here
    }
}