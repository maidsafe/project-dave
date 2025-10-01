use autonomi::client::payment::Receipt;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const PAYMENT_EXPIRATION_SECS: u64 = 3600 * 24 * 30; // 30 days

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPayment {
    pub receipt: Receipt,
    pub file_hash: String,
    pub timestamp: u64,
}

pub struct PaymentCache {
    cache_dir: PathBuf,
}

impl PaymentCache {
    pub fn new(base_dir: &Path) -> Result<Self, std::io::Error> {
        let cache_dir = base_dir.join("payments");
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    pub fn save_payment(&self, file_path: &Path, receipt: &Receipt) -> Result<(), std::io::Error> {
        let file_hash = Self::compute_file_hash(file_path)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let cached_payment = CachedPayment {
            receipt: receipt.clone(),
            file_hash: file_hash.clone(),
            timestamp,
        };

        let cache_filename = format!("{}_{}", timestamp, file_hash);
        let cache_path = self.cache_dir.join(cache_filename);

        let json = serde_json::to_string_pretty(&cached_payment)?;
        fs::write(cache_path, json)?;

        Ok(())
    }

    pub fn load_payment_for_file(&self, file_path: &Path) -> Result<Option<Receipt>, std::io::Error> {
        self.cleanup_outdated_payments()?;

        let target_hash = Self::compute_file_hash(file_path)?;

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                continue;
            }

            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(cached_payment) = serde_json::from_str::<CachedPayment>(&contents) {
                    if cached_payment.file_hash == target_hash {
                        let current_time = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);

                        if current_time - cached_payment.timestamp < PAYMENT_EXPIRATION_SECS {
                            return Ok(Some(cached_payment.receipt));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn cleanup_outdated_payments(&self) -> Result<(), std::io::Error> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(timestamp_str) = filename.split('_').next() {
                    if let Ok(timestamp) = timestamp_str.parse::<u64>() {
                        if current_time - timestamp > PAYMENT_EXPIRATION_SECS {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn compute_file_hash(file_path: &Path) -> Result<String, std::io::Error> {
        let file_content = fs::read(file_path)?;
        let mut hasher = DefaultHasher::new();
        file_content.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }
    
    pub fn save_archive_payment(&self, files: &[crate::ant::files::File], archive_name: &str, receipt: &Receipt) -> Result<(), std::io::Error> {
        let archive_hash = Self::compute_archive_hash(files, archive_name)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let cached_payment = CachedPayment {
            receipt: receipt.clone(),
            file_hash: archive_hash.clone(),
            timestamp,
        };

        let cache_filename = format!("{}_{}_archive", timestamp, archive_hash);
        let cache_path = self.cache_dir.join(cache_filename);

        let json = serde_json::to_string_pretty(&cached_payment)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(cache_path, json)?;

        Ok(())
    }
    
    pub fn load_archive_payment(&self, files: &[crate::ant::files::File], archive_name: &str) -> Result<Option<Receipt>, std::io::Error> {
        self.cleanup_outdated_payments()?;

        let target_hash = Self::compute_archive_hash(files, archive_name)?;

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.contains("_archive") {
                    if let Ok(contents) = fs::read_to_string(&path) {
                        if let Ok(cached_payment) = serde_json::from_str::<CachedPayment>(&contents) {
                            if cached_payment.file_hash == target_hash {
                                let current_time = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);

                                if current_time - cached_payment.timestamp < PAYMENT_EXPIRATION_SECS {
                                    return Ok(Some(cached_payment.receipt));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }
    
    fn compute_archive_hash(files: &[crate::ant::files::File], archive_name: &str) -> Result<String, std::io::Error> {
        let mut hasher = DefaultHasher::new();
        
        // Hash the archive name
        archive_name.hash(&mut hasher);
        
        // Collect all actual files (expand directories)
        let mut all_file_paths = Vec::new();
        for file in files {
            if let Ok(metadata) = fs::metadata(&file.path) {
                if metadata.is_dir() {
                    // Recursively collect files from directory
                    Self::collect_files_recursive(&file.path, &mut all_file_paths)?;
                } else {
                    // Single file
                    all_file_paths.push(file.path.clone());
                }
            }
        }
        
        // Sort files by path to ensure consistent hashing
        all_file_paths.sort();
        
        // Hash each file's content and metadata
        for file_path in all_file_paths {
            // Hash file path for structure
            file_path.to_string_lossy().hash(&mut hasher);
            
            // Hash file content (only for actual files, not directories)
            if let Ok(metadata) = fs::metadata(&file_path) {
                if metadata.is_file() {
                    let file_content = fs::read(&file_path)?;
                    file_content.hash(&mut hasher);
                    
                    // Hash file metadata for size/modification tracking
                    metadata.len().hash(&mut hasher);
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                            duration.as_secs().hash(&mut hasher);
                        }
                    }
                }
            }
        }
        
        Ok(format!("{:x}", hasher.finish()))
    }
    
    fn collect_files_recursive(dir_path: &std::path::Path, file_paths: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                Self::collect_files_recursive(&path, file_paths)?;
            } else if path.is_file() {
                file_paths.push(path);
            }
        }
        Ok(())
    }

    pub fn clear_cache(&self) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let _ = fs::remove_file(entry.path());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_payment_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache = PaymentCache::new(temp_dir.path()).unwrap();
        
        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, b"test content").unwrap();
        
        // Create a dummy receipt
        let receipt = Receipt::default();
        
        // Save payment
        cache.save_payment(&test_file, &receipt).unwrap();
        
        // Load payment
        let loaded = cache.load_payment_for_file(&test_file).unwrap();
        assert!(loaded.is_some());
    }
}