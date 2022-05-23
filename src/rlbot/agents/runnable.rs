pub trait Runnable {
    fn get_config_file_name(&self) -> &str;
    fn get_requirements_file(&self) -> &Option<String>;
    fn use_virtual_environment(&self) -> bool;
    fn get_environment_path(&self) -> String;
    fn calculate_missing_packages(&mut self);
}
