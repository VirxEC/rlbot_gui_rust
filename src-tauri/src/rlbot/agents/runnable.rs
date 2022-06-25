pub trait Runnable {
    fn get_config_file_name(&self) -> &str;
    fn get_requirements_file(&self) -> &Option<String>;
    fn use_virtual_environment(&self) -> bool;
    fn get_environment_path(&self) -> String;
    fn get_missing_packages(&self) -> Vec<String>;
    fn get_logo(&self) -> Option<String>;
}
