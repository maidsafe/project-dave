//! Persistent application data (settings).

use std::io::Write as _;
#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::{fs, io::Read as _};

use autonomi::Multiaddr;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use tracing::info;

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "autonomi";
const APPLICATION: &str = "dave";
const FILENAME_SETTINGS: &str = "settings.toml";

#[derive(ThisError, Debug)]
pub enum LoadError {
    #[error("Could not get home directory")]
    NoValidHome,
    #[error("Could not open file")]
    Open(OpenFileError),
    #[error("Could not read file")]
    Read(std::io::Error),
    #[error("Could not deserialize file")]
    Toml(toml::de::Error),
}

#[derive(ThisError, Debug)]
pub enum StoreError {
    #[error("Could not get home directory")]
    NoValidHome,
    #[error("Could not open file")]
    Open(OpenFileError),
    #[error("Could not write: file is read-only")]
    ReadOnly,
    #[error("Could not write to file")]
    Write(std::io::Error),
    #[error("Could not serialize file")]
    Toml(toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    pub download_path: Option<PathBuf>,
    pub peers: Option<Vec<Multiaddr>>,
    pub use_paymaster: Option<bool>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            download_path: directories::UserDirs::new()
                .and_then(|d| d.download_dir().map(|d| d.to_owned())),
            peers: None,
            use_paymaster: Some(false),
        }
    }
}

impl AppData {
    pub fn load() -> Result<Self, LoadError> {
        let filepath = filepath().ok_or(LoadError::NoValidHome)?;
        let mut file =
            open_file(&filepath, fs::OpenOptions::new().read(true)).map_err(LoadError::Open)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(LoadError::Read)?;

        let data: AppData = toml::from_str(&contents).map_err(LoadError::Toml)?;
        info!("loaded app data from: `{}`", filepath.display());

        Ok(data)
    }

    pub fn store(&self) -> Result<(), StoreError> {
        let filepath = filepath().ok_or(StoreError::NoValidHome)?;
        let mut file = open_file(
            &filepath,
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true),
        )
        .map_err(StoreError::Open)?;

        if file
            .metadata()
            .map_err(|_err| StoreError::ReadOnly)?
            .permissions()
            .readonly()
        {
            return Err(StoreError::ReadOnly);
        }

        let data = toml::to_string(&self).map_err(StoreError::Toml)?;
        file.write_all(data.as_bytes()).map_err(StoreError::Write)?;
        info!("stored app data to: `{}`", filepath.display());

        Ok(())
    }
}

#[derive(ThisError, Debug)]
pub enum OpenFileError {
    #[error("Failed to create (parent) directories for file")]
    CreateAllDirs(std::io::Error),
    #[error("Querying metadata for file failed")]
    QueryMetadata(std::io::Error),
    #[cfg(unix)]
    #[error("Failed to set permissions for file")]
    SetPermissions(std::io::Error),
    #[error("Failed to open file")]
    Open(std::io::Error),
}

fn open_file<P: AsRef<Path>>(path: P, oo: &fs::OpenOptions) -> Result<fs::File, OpenFileError> {
    let mut oo = oo.clone();
    let file_path = path.as_ref().to_path_buf();

    // Create the parent directory if it does not exist.
    let base = file_path
        .parent()
        .expect("config file path has no parent directory");
    if !base.exists() {
        fs::create_dir_all(base).map_err(OpenFileError::CreateAllDirs)?;
    }

    #[cfg(unix)]
    {
        // Put user-only permissions on config directory.
        let mut perm = base
            .metadata()
            .map_err(OpenFileError::QueryMetadata)?
            .permissions();
        if perm.mode() != 0o700 {
            perm.set_mode(0o700);

            fs::set_permissions(base, perm).map_err(OpenFileError::SetPermissions)?;
        }

        // Only allow user to read/write config file.
        oo.mode(0o600);
    }

    oo.open(&file_path).map_err(OpenFileError::Open)
}

fn filepath() -> Option<PathBuf> {
    let mut filepath = directories::ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)?
        .data_dir()
        .to_owned();
    filepath.push(FILENAME_SETTINGS);

    Some(filepath)
}

pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
        .map(|dirs| dirs.data_dir().to_owned())
}
