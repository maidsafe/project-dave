//! Local storage for file archives and addresses, similar to ant-cli

use autonomi::client::files::archive_private::PrivateArchiveDataMap;
use autonomi::client::files::archive_public::ArchiveAddress;
use autonomi::data::DataAddress;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error as ThisError;
use crate::ant::client::SharedClient;
use crate::ant::files::PublicOrPrivateFile;
use tauri::{AppHandle, Emitter, State};

#[derive(ThisError, Debug)]
pub enum LocalStorageError {
    #[error("Could not get local data directory")]
    NoDataDir,
    #[error("Could not read file: {0}")]
    ReadError(std::io::Error),
    #[error("Could not write file: {0}")]
    WriteError(std::io::Error),
    #[error("Could not parse file: {0}")]
    ParseError(serde_json::Error),
    #[error("Could not parse hex: {0}")]
    HexError(String),
}

#[derive(Serialize, Deserialize)]
struct PrivateFileArchive {
    name: String,
    secret_access: String,
}

#[derive(Serialize, Deserialize)]
struct PrivateFile {
    name: String,
    secret_access: String,
}

#[derive(Serialize, Deserialize)]
struct PublicFile {
    name: String,
    data_address: String,
}

/// Get the local user data directory path - same as ant-cli
fn get_user_data_dir() -> Result<PathBuf, LocalStorageError> {
    let mut home_dirs = dirs_next::data_dir()
        .ok_or(LocalStorageError::NoDataDir)?;
    home_dirs.push("autonomi");
    home_dirs.push("client");
    home_dirs.push("user_data");
    
    fs::create_dir_all(&home_dirs).map_err(LocalStorageError::WriteError)?;
    Ok(home_dirs)
}

/// Write a public file archive to local storage
pub fn write_local_public_file_archive(
    archive: String,
    name: &str,
) -> Result<(), LocalStorageError> {
    let user_data_path = get_user_data_dir()?;
    let file_archives_path = user_data_path.join("file_archives");
    fs::create_dir_all(&file_archives_path).map_err(LocalStorageError::WriteError)?;
    fs::write(file_archives_path.join(archive), name).map_err(LocalStorageError::WriteError)?;
    Ok(())
}

/// Write a private file archive to local storage
pub fn write_local_private_file_archive(
    archive: String,
    local_addr: String,
    name: &str,
) -> Result<(), LocalStorageError> {
    let user_data_path = get_user_data_dir()?;
    let private_file_archives_path = user_data_path.join("private_file_archives");
    fs::create_dir_all(&private_file_archives_path).map_err(LocalStorageError::WriteError)?;

    let content = serde_json::to_string(&PrivateFileArchive {
        name: name.to_string(),
        secret_access: archive,
    })
    .map_err(LocalStorageError::ParseError)?;

    fs::write(private_file_archives_path.join(local_addr), content)
        .map_err(LocalStorageError::WriteError)?;
    Ok(())
}

/// Write a private file to local storage
pub fn write_local_private_file(
    datamap_hex: String,
    local_addr: String,
    name: &str,
) -> Result<(), LocalStorageError> {
    let user_data_path = get_user_data_dir()?;
    let private_files_path = user_data_path.join("private_files");
    fs::create_dir_all(&private_files_path).map_err(LocalStorageError::WriteError)?;

    let content = serde_json::to_string(&PrivateFile {
        name: name.to_string(),
        secret_access: datamap_hex,
    })
    .map_err(LocalStorageError::ParseError)?;

    fs::write(private_files_path.join(local_addr), content)
        .map_err(LocalStorageError::WriteError)?;
    Ok(())
}

/// Write a public file to local storage
pub fn write_local_public_file(data_address: String, name: &str) -> Result<(), LocalStorageError> {
    let user_data_path = get_user_data_dir()?;
    let public_files_path = user_data_path.join("public_files");
    fs::create_dir_all(&public_files_path).map_err(LocalStorageError::WriteError)?;

    let content = serde_json::to_string(&PublicFile {
        name: name.to_string(),
        data_address: data_address.clone(),
    })
    .map_err(LocalStorageError::ParseError)?;

    fs::write(public_files_path.join(data_address), content)
        .map_err(LocalStorageError::WriteError)?;
    Ok(())
}

/// Get all local public file archives
pub fn get_local_public_file_archives() -> Result<HashMap<ArchiveAddress, String>, LocalStorageError>
{
    let user_data_path = get_user_data_dir()?;
    let file_archives_path = user_data_path.join("file_archives");
    fs::create_dir_all(&file_archives_path).map_err(LocalStorageError::WriteError)?;

    let mut file_archives = HashMap::new();
    
    for entry in walkdir::WalkDir::new(file_archives_path)
        .min_depth(1)
        .max_depth(1)
    {
        let entry = entry.map_err(|e| LocalStorageError::ReadError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        let file_name = entry.file_name().to_string_lossy();
        if let Ok(file_archive_address) = DataAddress::from_hex(&file_name) {
            if let Ok(file_archive_name) = fs::read_to_string(entry.path()) {
                file_archives.insert(file_archive_address, file_archive_name);
            }
        }
    }

    Ok(file_archives)
}

/// Get all local private file archives
pub fn get_local_private_file_archives(
) -> Result<HashMap<PrivateArchiveDataMap, String>, LocalStorageError> {
    let user_data_path = get_user_data_dir()?;
    let private_file_archives_path = user_data_path.join("private_file_archives");
    fs::create_dir_all(&private_file_archives_path).map_err(LocalStorageError::WriteError)?;

    let mut private_file_archives = HashMap::new();
    
    for entry in walkdir::WalkDir::new(private_file_archives_path)
        .min_depth(1)
        .max_depth(1)
    {
        let entry = entry.map_err(|e| LocalStorageError::ReadError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        if let Ok(file_content) = fs::read_to_string(entry.path()) {
            if let Ok(private_file_archive) =
                serde_json::from_str::<PrivateFileArchive>(&file_content)
            {
                if let Ok(private_file_archive_access) =
                    PrivateArchiveDataMap::from_hex(&private_file_archive.secret_access)
                {
                    private_file_archives
                        .insert(private_file_archive_access, private_file_archive.name);
                }
            }
        }
    }

    Ok(private_file_archives)
}

/// Get all local public files
pub fn get_local_public_files() -> Result<HashMap<DataAddress, String>, LocalStorageError> {
    let user_data_path = get_user_data_dir()?;
    let public_files_path = user_data_path.join("public_files");
    let mut files = HashMap::new();

    if !public_files_path.exists() {
        return Ok(files);
    }

    for entry in walkdir::WalkDir::new(public_files_path)
        .min_depth(1)
        .max_depth(1)
    {
        let entry = entry.map_err(|e| LocalStorageError::ReadError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        if let Ok(content) = fs::read_to_string(entry.path()) {
            if let Ok(public_file) = serde_json::from_str::<PublicFile>(&content) {
                if let Ok(data_address) = DataAddress::from_hex(&public_file.data_address) {
                    files.insert(data_address, public_file.name);
                }
            }
        }
    }

    Ok(files)
}

/// Get all local private files
pub fn get_local_private_files() -> Result<HashMap<PrivateArchiveDataMap, String>, LocalStorageError>
{
    let user_data_path = get_user_data_dir()?;
    let private_files_path = user_data_path.join("private_files");
    let mut files = HashMap::new();

    if !private_files_path.exists() {
        return Ok(files);
    }

    for entry in walkdir::WalkDir::new(private_files_path)
        .min_depth(1)
        .max_depth(1)
    {
        let entry = entry.map_err(|e| LocalStorageError::ReadError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        if let Ok(content) = fs::read_to_string(entry.path()) {
            if let Ok(private_file) = serde_json::from_str::<PrivateFile>(&content) {
                if let Ok(datamap) =
                    PrivateArchiveDataMap::from_hex(&private_file.secret_access)
                {
                    files.insert(datamap, private_file.name);
                }
            }
        }
    }

    Ok(files)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalFileData {
    pub public_file_archives: Vec<LocalArchive>,
    pub private_file_archives: Vec<LocalArchive>,
    pub public_files: Vec<LocalFile>,
    pub private_files: Vec<LocalFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalArchive {
    pub name: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalFile {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalUpdate {
    pub update_type: LocalUpdateType,
    pub archive: Option<LocalArchiveLoaded>,
    pub failed_archive: Option<LocalFailedArchive>,
    pub loading_archive: Option<LocalLoadingArchive>,
    pub files: Vec<LocalIndividualFile>,
    pub is_complete: bool,
    pub temp_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LocalUpdateType {
    IndividualFiles,
    ArchiveLoading,
    ArchiveLoaded,
    ArchiveFailed,
    Complete,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalArchiveLoaded {
    pub name: String,
    pub address: String,
    pub is_private: bool,
    pub files: Vec<LocalFileInArchive>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalFailedArchive {
    pub name: String,
    pub address: String,
    pub is_private: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalLoadingArchive {
    pub name: String,
    pub address: String,
    pub is_private: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalIndividualFile {
    pub name: String,
    pub address: String,
    pub is_private: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalFileInArchive {
    pub path: String,
    pub metadata: autonomi::files::Metadata,
    pub file_access: PublicOrPrivateFile,
    pub is_private: bool,
}

/// Get archive access data for loading its contents
pub fn get_local_private_archive_access(local_addr: &str) -> Result<PrivateArchiveDataMap, LocalStorageError> {
    let user_data_path = get_user_data_dir()?;
    let private_file_archives_path = user_data_path.join("private_file_archives");
    let file_path = private_file_archives_path.join(local_addr);
    
    let file_content = fs::read_to_string(file_path).map_err(LocalStorageError::ReadError)?;
    let private_file_archive: PrivateFileArchive = serde_json::from_str(&file_content)
        .map_err(LocalStorageError::ParseError)?;
    let private_file_archive_access = PrivateArchiveDataMap::from_hex(&private_file_archive.secret_access)
        .map_err(|e| LocalStorageError::HexError(e.to_string()))?;
        
    Ok(private_file_archive_access)
}

/// Get public archive address for loading its contents  
pub fn get_local_public_archive_address(address_hex: &str) -> Result<ArchiveAddress, LocalStorageError> {
    DataAddress::from_hex(address_hex).map_err(|e| LocalStorageError::HexError(e.to_string()))
}

/// Get all local file data in a structured format
pub fn get_all_local_files() -> Result<LocalFileData, LocalStorageError> {
    let mut public_file_archives = Vec::new();
    for (addr, name) in get_local_public_file_archives()? {
        public_file_archives.push(LocalArchive {
            name,
            address: addr.to_hex(),
        });
    }

    let mut private_file_archives = Vec::new();
    for (datamap, name) in get_local_private_file_archives()? {
        private_file_archives.push(LocalArchive {
            name,
            address: datamap.address(),
        });
    }

    let mut public_files = Vec::new();
    for (addr, name) in get_local_public_files()? {
        public_files.push(LocalFile {
            name,
            address: addr.to_hex(),
        });
    }

    let mut private_files = Vec::new();
    for (datamap, name) in get_local_private_files()? {
        private_files.push(LocalFile {
            name,
            address: datamap.address(),
        });
    }

    Ok(LocalFileData {
        public_file_archives,
        private_file_archives,
        public_files,
        private_files,
    })
}

/// Get local file structure with streaming updates similar to vault files
pub async fn get_local_structure_streaming(
    app: AppHandle,
    temp_code: String,
    shared_client: State<'_, SharedClient>,
) -> Result<(), LocalStorageError> {
    let client = shared_client.get_client().await
        .map_err(|e| LocalStorageError::ReadError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    // Get local file data first
    let local_data = get_all_local_files()?;

    // First, emit individual files immediately (these are already available locally)
    let mut individual_files: Vec<LocalIndividualFile> = vec![];

    // Process individual public files
    for file in &local_data.public_files {
        individual_files.push(LocalIndividualFile {
            name: file.name.clone(),
            address: file.address.clone(),
            is_private: false,
        });
    }

    // Process individual private files
    for file in &local_data.private_files {
        individual_files.push(LocalIndividualFile {
            name: file.name.clone(),
            address: file.address.clone(),
            is_private: true,
        });
    }

    // Emit individual files first if we have any
    if !individual_files.is_empty() {
        let update = LocalUpdate {
            update_type: LocalUpdateType::IndividualFiles,
            archive: None,
            failed_archive: None,
            loading_archive: None,
            files: individual_files,
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        app.emit("local-update", update).map_err(|e| LocalStorageError::WriteError(
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        ))?;
    }

    // Process archives concurrently
    let mut archive_tasks = vec![];

    // Create tasks for private archives
    for archive in local_data.private_file_archives {
        let client = client.clone();
        let app = app.clone();
        let archive_name = archive.name.clone();
        let archive_address = archive.address.clone();
        
        // Emit loading status immediately
        let loading_update = LocalUpdate {
            update_type: LocalUpdateType::ArchiveLoading,
            archive: None,
            failed_archive: None,
            loading_archive: Some(LocalLoadingArchive {
                name: archive_name.clone(),
                address: archive_address.clone(),
                is_private: true,
            }),
            files: vec![],
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        let _ = app.emit("local-update", loading_update);
        
        let temp_code = temp_code.clone();
        let task = tokio::spawn(async move {
            match get_local_private_archive_access(&archive_address) {
                Ok(archive_datamap) => {
                    match client.archive_get(&archive_datamap).await {
                        Ok(archive) => {
                            let mut files: Vec<LocalFileInArchive> = vec![];
                            
                            for (filepath, (data_map, metadata)) in archive.map() {
                                files.push(LocalFileInArchive {
                                    path: filepath.display().to_string(),
                                    metadata: metadata.clone(),
                                    file_access: PublicOrPrivateFile::Private(data_map.clone()),
                                    is_private: true,
                                });
                            }

                            let archive_loaded = LocalArchiveLoaded {
                                name: archive_name.clone(),
                                address: archive_address.clone(),
                                is_private: true,
                                files,
                            };

                            let update = LocalUpdate {
                                update_type: LocalUpdateType::ArchiveLoaded,
                                archive: Some(archive_loaded),
                                failed_archive: None,
                                loading_archive: None,
                                files: vec![],
                                is_complete: false,
                                temp_code: temp_code.clone(),
                            };

                            let _ = app.emit("local-update", update);
                        }
                        Err(_) => {
                            let failed_archive = LocalFailedArchive {
                                name: archive_name.clone(),
                                address: archive_address.clone(),
                                is_private: true,
                            };

                            let update = LocalUpdate {
                                update_type: LocalUpdateType::ArchiveFailed,
                                archive: None,
                                failed_archive: Some(failed_archive),
                                loading_archive: None,
                                files: vec![],
                                is_complete: false,
                                temp_code: temp_code.clone(),
                            };

                            let _ = app.emit("local-update", update);
                        }
                    }
                }
                Err(_) => {
                    let failed_archive = LocalFailedArchive {
                        name: archive_name.clone(),
                        address: archive_address.clone(),
                        is_private: true,
                    };

                    let update = LocalUpdate {
                        update_type: LocalUpdateType::ArchiveFailed,
                        archive: None,
                        failed_archive: Some(failed_archive),
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("local-update", update);
                }
            }
        });
        archive_tasks.push(task);
    }

    // Create tasks for public archives
    for archive in local_data.public_file_archives {
        let client = client.clone();
        let app = app.clone();
        let archive_name = archive.name.clone();
        let archive_address = archive.address.clone();
        
        // Emit loading status immediately
        let loading_update = LocalUpdate {
            update_type: LocalUpdateType::ArchiveLoading,
            archive: None,
            failed_archive: None,
            loading_archive: Some(LocalLoadingArchive {
                name: archive_name.clone(),
                address: archive_address.clone(),
                is_private: false,
            }),
            files: vec![],
            is_complete: false,
            temp_code: temp_code.clone(),
        };
        let _ = app.emit("local-update", loading_update);
        
        let temp_code = temp_code.clone();
        let task = tokio::spawn(async move {
            match get_local_public_archive_address(&archive_address) {
                Ok(archive_addr) => {
                    match client.archive_get_public(&archive_addr).await {
                        Ok(archive) => {
                            let mut files: Vec<LocalFileInArchive> = vec![];
                            
                            for (filepath, (data_addr, metadata)) in archive.map() {
                                files.push(LocalFileInArchive {
                                    path: filepath.display().to_string(),
                                    metadata: metadata.clone(),
                                    file_access: PublicOrPrivateFile::Public(*data_addr),
                                    is_private: false,
                                });
                            }

                            let archive_loaded = LocalArchiveLoaded {
                                name: archive_name.clone(),
                                address: archive_address.clone(),
                                is_private: false,
                                files,
                            };

                            let update = LocalUpdate {
                                update_type: LocalUpdateType::ArchiveLoaded,
                                archive: Some(archive_loaded),
                                failed_archive: None,
                                loading_archive: None,
                                files: vec![],
                                is_complete: false,
                                temp_code: temp_code.clone(),
                            };

                            let _ = app.emit("local-update", update);
                        }
                        Err(_) => {
                            let failed_archive = LocalFailedArchive {
                                name: archive_name.clone(),
                                address: archive_address.clone(),
                                is_private: false,
                            };

                            let update = LocalUpdate {
                                update_type: LocalUpdateType::ArchiveFailed,
                                archive: None,
                                failed_archive: Some(failed_archive),
                                loading_archive: None,
                                files: vec![],
                                is_complete: false,
                                temp_code: temp_code.clone(),
                            };

                            let _ = app.emit("local-update", update);
                        }
                    }
                }
                Err(_) => {
                    let failed_archive = LocalFailedArchive {
                        name: archive_name.clone(),
                        address: archive_address.clone(),
                        is_private: false,
                    };

                    let update = LocalUpdate {
                        update_type: LocalUpdateType::ArchiveFailed,
                        archive: None,
                        failed_archive: Some(failed_archive),
                        loading_archive: None,
                        files: vec![],
                        is_complete: false,
                        temp_code: temp_code.clone(),
                    };

                    let _ = app.emit("local-update", update);
                }
            }
        });
        archive_tasks.push(task);
    }

    // Wait for all archive tasks to complete
    for task in archive_tasks {
        let _ = task.await;
    }

    // Emit completion
    let completion_update = LocalUpdate {
        update_type: LocalUpdateType::Complete,
        archive: None,
        failed_archive: None,
        loading_archive: None,
        files: vec![],
        is_complete: true,
        temp_code: temp_code.clone(),
    };
    app.emit("local-update", completion_update).map_err(|e| LocalStorageError::WriteError(
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    ))?;

    Ok(())
}
