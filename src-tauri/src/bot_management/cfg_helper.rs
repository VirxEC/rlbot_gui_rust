use configparser::ini::Ini;
use std::path::Path;
use thiserror::Error;
use tokio::fs as async_fs;

use crate::impl_serialize_from_display;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Could not load cfg: {0}")]
    Load(String),
    #[error("I/O error when managing cfg: {0}")]
    Io(#[from] std::io::Error),
}

impl_serialize_from_display!(Error);

/// Load a CFG file synchronously, returns a description of any errors if unable to do so
///
/// # Arguments
///
/// * `path`: The path to the CFG file that needs to be loaded
pub fn load_cfg_sync<T: AsRef<Path>>(path: T) -> Result<Ini, Error> {
    let mut conf = Ini::new();
    conf.set_multiline(true);
    conf.set_comment_symbols(&[';']);
    conf.load(path).map_err(Error::Load)?;

    Ok(conf)
}

/// Load a CFG file, returns a description of any errors if unable to do so
///
/// # Arguments
///
/// * `path`: The path to the CFG file that needs to be loaded
pub async fn load_cfg<T: AsRef<Path>>(path: T) -> Result<Ini, Error> {
    let mut conf = Ini::new();
    conf.set_multiline(true);
    conf.set_comment_symbols(&[';']);
    conf.read(async_fs::read_to_string(path).await?).map_err(Error::Load)?;

    Ok(conf)
}

/// Save a CFG file, returns a description of any errors if unable to do so
///
/// # Arguments
///
/// * `conf`: The CFG file that needs to be saved
/// * `path`: Where to save the CFG file to
pub async fn save_cfg<T: AsRef<Path>>(conf: &Ini, path: T) -> Result<(), Error> {
    async_fs::write(path, conf.writes()).await?;
    Ok(())
}

/// Load, change a key, and save a cfg file. Returns a descripton of any errors if unable to do so
///
/// # Arguments
///
/// * `path`: The path to the CFG file
/// * `section`: The section of the CFG file to change
/// * `key`: The key in `section` to change
/// * `value`: What to set the value to
pub async fn change_key_in_cfg<T: AsRef<Path>>(path: T, section: &str, key: &str, value: String) -> Result<(), Error> {
    let mut conf = load_cfg(&path).await?;
    conf.set(section, key, Some(value));
    save_cfg(&conf, path).await
}
