// pub const PYTHON_FILE_KEY: &str = "python_file";
// pub const REQUIREMENTS_FILE_KEY: &str = "requirements_file";
// pub const LOGO_FILE_KEY: &str = "logo_file";
// pub const NAME_KEY: &str = "name";
// pub const SUPPORTS_EARLY_START_KEY: &str = "supports_early_start";
// pub const REQUIRES_TKINTER: &str = "requires_tkinter";
// pub const USE_VIRTUAL_ENVIRONMENT_KEY: &str = "use_virtual_environment";

pub trait Runnable {
    fn get_config_file_name(&self) -> &str;
}
