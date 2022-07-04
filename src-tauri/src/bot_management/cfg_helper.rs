use configparser::ini::Ini;
use std::path::Path;

/// Load a CFG file, returns a description of any errors if unable to do so
///
/// # Arguments
///
/// * `path`: The path to the CFG file that needs to be loaded
pub fn load_cfg<T: AsRef<Path>>(path: T) -> Result<Ini, String> {
    let mut conf = Ini::new();
    conf.set_comment_symbols(&[';']);

    if let Err(e) = conf.load(path) {
        Err(format!("Failed to load config file: {}", e))
    } else {
        Ok(conf)
    }
}

/// Save a CFG file, returns a description of any errors if unable to do so
pub fn save_cfg<T: AsRef<Path>>(conf: Ini, path: T) -> Result<(), String> {
    if let Err(e) = conf.write(path) {
        Err(format!("Failed to write config file: {}", e))
    } else {
        Ok(())
    }
}

/// Load, change a key, and save a cfg file. Returns a descripton of any errors if unable to do so
pub fn change_key_in_cfg<T: AsRef<Path>>(path: T, section: &str, key: &str, value: String) -> Result<(), String> {
    let mut conf = load_cfg(&path)?;
    conf.set(section, key, Some(value));
    save_cfg(conf, path)
}
