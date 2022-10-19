use std::ffi::OsStr;

use tauri::Window;

pub trait Runnable {
    fn get_config_file_name(&self) -> &str;
    fn get_requirements_file(&self) -> &Option<String>;
    fn use_virtual_environment(&self) -> bool;
    fn get_missing_packages<S: AsRef<OsStr>>(&self, window: &Window, python: S) -> Vec<String>;
    fn logo(&self) -> &Option<String>;
    fn load_logo(&self) -> Option<String>;
    fn is_rlbot_controlled(&self) -> bool;
    fn warn(&self) -> &Option<String>;
    fn missing_python_packages(&self) -> &Option<Vec<String>>;
    fn may_require_python_packages(&self) -> bool;
}
