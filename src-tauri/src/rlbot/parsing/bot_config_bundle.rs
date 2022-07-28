use crate::{
    bot_management::cfg_helper::load_cfg,
    ccprintln, get_command_status,
    rlbot::agents::{base_script::SCRIPT_FILE_KEY, runnable::Runnable},
    PYTHON_PATH,
};
use configparser::ini::Ini;
use imghdr::Type;
use serde::{Deserialize, Serialize};
use std::{
    borrow::ToOwned,
    fs,
    io::Read,
    path::Path,
    process::{self, Stdio},
    str::from_utf8,
};
use tauri::Window;

pub const PYTHON_FILE_KEY: &str = "python_file";
pub const REQUIREMENTS_FILE_KEY: &str = "requirements_file";
pub const LOGO_FILE_KEY: &str = "logo_file";
pub const NAME_KEY: &str = "name";
// pub const SUPPORTS_EARLY_START_KEY: &str = "supports_early_start";
pub const REQUIRES_TKINTER: &str = "requires_tkinter";
pub const USE_VIRTUAL_ENVIRONMENT_KEY: &str = "use_virtual_environment";

pub const BOT_CONFIG_MODULE_HEADER: &str = "Locations";
pub const BOT_CONFIG_DETAILS_HEADER: &str = "Details";
pub const SUPPORTS_STANDALONE: &str = "supports_standalone";
// pub const LOADOUT_GENERATOR_FILE_KEY: &str = "loadout_generator";
pub const LOOKS_CONFIG_KEY: &str = "looks_config";
// pub const SUPPORTS_EARLY_START_KEY: &str = "supports_early_start";
// pub const MAXIMUM_TICK_RATE_PREFERENCE_KEY: &str = "maximum_tick_rate_preference";

pub const BOT_CONFIG_PARAMS_HEADER: &str = "Bot Parameters";
pub const EXECUTABLE_PATH_KEY: &str = "path";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DevInfo {
    pub developer: String,
    pub description: String,
    pub fun_fact: String,
    pub github: String,
    pub language: String,
    pub tags: Vec<String>,
}

impl DevInfo {
    pub fn from_config(config: &Ini) -> Self {
        let developer = config.get(BOT_CONFIG_DETAILS_HEADER, "developer").unwrap_or_default();
        let description = config.get(BOT_CONFIG_DETAILS_HEADER, "description").unwrap_or_default();
        let fun_fact = config.get(BOT_CONFIG_DETAILS_HEADER, "fun_fact").unwrap_or_default();
        let github = config.get(BOT_CONFIG_DETAILS_HEADER, "github").unwrap_or_default();
        let language = config.get(BOT_CONFIG_DETAILS_HEADER, "language").unwrap_or_default();
        let tags = config
            .get(BOT_CONFIG_DETAILS_HEADER, "tags")
            .unwrap_or_default()
            .split(", ")
            .map(ToOwned::to_owned)
            .collect();

        Self {
            developer,
            description,
            fun_fact,
            github,
            language,
            tags,
        }
    }
}

fn get_file_extension(vec: &[u8]) -> Option<&'static str> {
    match imghdr::from_bytes(vec) {
        // Gif 87a and 89a Files
        Some(Type::Gif) => Some("gif"),
        // TIFF files
        Some(Type::Tiff) => Some("tiff"),
        // Sun Raster files
        Some(Type::Rast) => Some("ras"),
        // X Bitmap files
        Some(Type::Xbm) => Some("xbm"),
        // JPEG data in JFIF or Exif formats
        Some(Type::Jpeg) => Some("jpeg"),
        // BMP files
        Some(Type::Bmp) => Some("bmp"),
        // Portable Network Graphics
        Some(Type::Png) => Some("png"),
        // WebP files
        Some(Type::Webp) => Some("webp"),
        // OpenEXR files
        Some(Type::Exr) => Some("exr"),
        // BGP (Better Portable Graphics) files
        Some(Type::Bgp) => Some("bgp"),
        // PBM (Portable bitmap) files
        Some(Type::Pbm) => Some("pbm"),
        // PGM (Portable graymap) files
        Some(Type::Pgm) => Some("pgm"),
        // PPM (Portable pixmap) files
        Some(Type::Ppm) => Some("ppm"),
        // SGI image library files
        Some(Type::Rgb) => Some("rgb"),
        // HDR files (RGBE)
        Some(Type::Rgbe) => Some("hdr"),
        // FLIF (Free Lossless Image Format) files
        Some(Type::Flif) => Some("flif"),
        // ICO files
        Some(Type::Ico) => Some("ico"),
        None => None,
    }
}

pub fn to_base64(path: &str) -> Option<String> {
    if let Ok(file) = &mut fs::File::open(path) {
        let mut vec = Vec::new();
        file.read_to_end(&mut vec).ok()?;

        get_file_extension(&vec).map(|extension| format!("data:image/{};base64,{}", extension, base64::encode(vec).replace("\r\n", "")))
    } else {
        None
    }
}

pub trait Clean {
    fn cleaned(&self) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub struct BotConfigBundle {
    pub name: Option<String>,
    pub looks_path: Option<String>,
    pub path: Option<String>,
    config_file_name: Option<String>,
    config_directory: Option<String>,
    pub info: Option<DevInfo>,
    pub logo_path: Option<String>,
    pub logo: Option<String>,
    pub runnable_type: String,
    pub warn: Option<String>,
    pub image: String,
    supports_standalone: Option<bool>,
    use_virtual_environment: Option<bool>,
    requirements_file: Option<String>,
    requires_tkinter: Option<bool>,
    pub missing_python_packages: Option<Vec<String>>,
    pub python_path: Option<String>,
}

impl BotConfigBundle {
    pub fn minimal_from_path(config_path: &Path) -> Result<Self, String> {
        let conf = load_cfg(config_path)?;

        let path = config_path.to_string_lossy().to_string();
        let config_directory = config_path.parent().unwrap().to_string_lossy().to_string();
        let config_file_name = Some(config_path.file_name().unwrap().to_string_lossy().to_string());

        let name = conf.get(BOT_CONFIG_MODULE_HEADER, NAME_KEY);
        let looks_path = conf.get(BOT_CONFIG_MODULE_HEADER, LOOKS_CONFIG_KEY).map(|path| format!("{}/{}", config_directory, path));
        let python_path = conf.get(BOT_CONFIG_MODULE_HEADER, PYTHON_FILE_KEY).map(|path| format!("{}/{}", config_directory, path));
        let supports_standalone = conf.get(BOT_CONFIG_MODULE_HEADER, SUPPORTS_STANDALONE).map(|s| s.parse::<bool>().unwrap_or_default());
        let use_virtual_environment = conf.getbool(BOT_CONFIG_MODULE_HEADER, USE_VIRTUAL_ENVIRONMENT_KEY).unwrap_or(None);
        let requirements_file = conf
            .get(BOT_CONFIG_MODULE_HEADER, REQUIREMENTS_FILE_KEY)
            .map(|path| format!("{}/{}", config_directory, path));
        let requires_tkinter = conf.getbool(BOT_CONFIG_MODULE_HEADER, REQUIRES_TKINTER).unwrap_or(Some(false));

        if name.is_none() {
            return Err("Bot name not found".to_owned());
        }

        let valid_looks = match &looks_path {
            Some(path) => Path::new(path).exists(),
            None => false,
        };

        if !valid_looks {
            return Err("Looks config not found".to_owned());
        }

        let valid_path = match &python_path {
            Some(path) => Path::new(path).exists(),
            None => false,
        };

        if !valid_path {
            return Err("Python file not found".to_owned());
        }

        let relative_logo_path = conf.get(BOT_CONFIG_MODULE_HEADER, LOGO_FILE_KEY).unwrap_or_else(|| String::from("logo.png"));
        let absolute_logo_path = format!("{}/{}", config_directory, relative_logo_path);

        let logo = None;

        let info = Some(DevInfo::from_config(&conf));

        let runnable_type = String::from("rlbot");
        let warn = None;
        let image = String::from("imgs/rlbot.png");
        let missing_python_packages = None;

        let path = Some(path);
        let logo_path = Some(absolute_logo_path);
        let config_directory = Some(config_directory);

        Ok(Self {
            name,
            looks_path,
            path,
            config_file_name,
            config_directory,
            info,
            logo_path,
            logo,
            runnable_type,
            warn,
            image,
            supports_standalone,
            use_virtual_environment,
            requirements_file,
            requires_tkinter,
            missing_python_packages,
            python_path,
        })
    }

    pub fn name_from_path(config_path: &Path) -> Result<(String, String), String> {
        let conf = load_cfg(config_path)?;

        let name = if let Some(the_name) = conf.get(BOT_CONFIG_MODULE_HEADER, NAME_KEY) {
            the_name
        } else {
            return Err("Bot name not found".to_owned());
        };

        let path = config_path.to_string_lossy().to_string();

        let config_directory = config_path.parent().unwrap();

        let looks_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, LOOKS_CONFIG_KEY)
            .map(|path| format!("{}/{}", config_directory.display(), path));

        let valid_looks = match &looks_path {
            Some(path) => Path::new(path).exists(),
            None => false,
        };

        if !valid_looks {
            return Err("Looks config not found".to_owned());
        }

        let python_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, PYTHON_FILE_KEY)
            .map(|path| format!("{}/{}", config_directory.display(), path));

        let valid_path = match &python_path {
            Some(path) => Path::new(path).exists(),
            None => false,
        };

        if !valid_path {
            return Err("Python file not found".to_owned());
        }

        Ok((name, path))
    }
}

impl Clean for BotConfigBundle {
    fn cleaned(&self) -> Self {
        let mut b = self.clone();
        b.logo = None;
        b.warn = None;
        b.missing_python_packages = None;
        b
    }
}

impl Runnable for BotConfigBundle {
    fn get_config_file_name(&self) -> &str {
        self.config_file_name.as_ref().unwrap()
    }

    fn get_requirements_file(&self) -> &Option<String> {
        &self.requirements_file
    }

    fn use_virtual_environment(&self) -> bool {
        self.supports_standalone.unwrap_or_default() && self.use_virtual_environment.unwrap_or_default()
    }

    fn get_missing_packages(&self, window: &Window) -> Vec<String> {
        if self.use_virtual_environment() {
            return Vec::new();
        }

        let python = if let Ok(path) = PYTHON_PATH.lock() {
            path.to_owned()
        } else {
            return Vec::new();
        };

        let requires_tkinter = self.requires_tkinter.unwrap_or_default();

        if let Some(req_file) = self.get_requirements_file() {
            let mut args: Vec<&str> = vec!["-c", "from rlbot_smh.get_missing_packages import run; run()"];

            if requires_tkinter {
                args.push("requires_tkinter");
            }

            let file = format!("requirements_file={}", req_file);

            args.push(&file);

            let mut command = process::Command::new(python);

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                // disable window creation
                command.creation_flags(0x08000000);
            };

            match command.args(args).stdin(Stdio::null()).output() {
                Ok(proc) => {
                    let output = from_utf8(proc.stdout.as_slice()).unwrap();
                    if let Ok(packages) = serde_json::from_str(output) {
                        return packages;
                    }
                }
                Err(e) => ccprintln(window, format!("Failed to calculate missing packages: {}", e)),
            }
        } else if requires_tkinter && !get_command_status(python, ["-c", "import tkinter"]) {
            return vec![String::from("tkinter")];
        }

        Vec::new()
    }

    fn logo(&self) -> &Option<String> {
        &self.logo
    }

    fn load_logo(&self) -> Option<String> {
        if let Some(logo_path) = &self.logo_path {
            to_base64(logo_path)
        } else {
            None
        }
    }

    fn is_rlbot_controlled(&self) -> bool {
        self.runnable_type == "rlbot"
    }

    fn warn(&self) -> &Option<String> {
        &self.warn
    }

    fn missing_python_packages(&self) -> &Option<Vec<String>> {
        &self.missing_python_packages
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ScriptConfigBundle {
    pub name: Option<String>,
    pub runnable_type: String,
    pub warn: Option<String>,
    pub image: String,
    pub path: String,
    pub info: Option<DevInfo>,
    pub logo: Option<String>,
    pub logo_path: Option<String>,
    pub missing_python_packages: Option<Vec<String>>,
    config_file_name: String,
    config_directory: String,
    script_file: String,
    use_virtual_environment: bool,
    requirements_file: Option<String>,
    requires_tkinter: bool,
}

impl ScriptConfigBundle {
    pub fn minimal_from_path(config_path: &Path) -> Result<Self, String> {
        let conf = load_cfg(config_path)?;

        let name = conf.get(BOT_CONFIG_MODULE_HEADER, NAME_KEY);
        let runnable_type = String::from("script");
        let warn = None;
        let image = String::from("imgs/rlbot.png");
        let path = config_path.to_string_lossy().to_string();
        let config_directory = config_path.parent().unwrap().to_string_lossy().to_string();
        let config_file_name = config_path.file_name().unwrap().to_string_lossy().to_string();
        let use_virtual_environment = conf.getbool(BOT_CONFIG_MODULE_HEADER, USE_VIRTUAL_ENVIRONMENT_KEY).unwrap_or(None).unwrap_or_default();
        let requirements_file = conf
            .get(BOT_CONFIG_MODULE_HEADER, REQUIREMENTS_FILE_KEY)
            .map(|path| format!("{}/{}", config_directory, path));
        let requires_tkinter = conf.getbool(BOT_CONFIG_MODULE_HEADER, REQUIRES_TKINTER).unwrap_or(None).unwrap_or_default();

        let script_file = conf
            .get(BOT_CONFIG_MODULE_HEADER, SCRIPT_FILE_KEY)
            .map(|path| format!("{}/{}", config_directory, path))
            .unwrap_or_default();

        if name.is_none() {
            return Err("Bot name not found".to_owned());
        }

        if !Path::new(&script_file).exists() {
            return Err("Script file not found".to_owned());
        }

        let relative_logo_path = conf.get(BOT_CONFIG_MODULE_HEADER, LOGO_FILE_KEY).unwrap_or_else(|| String::from("logo.png"));
        let absolute_logo_path = format!("{}/{}", config_directory, relative_logo_path);
        let logo = None;

        let info = Some(DevInfo::from_config(&conf));

        let missing_python_packages = None;
        let logo_path = Some(absolute_logo_path);

        Ok(Self {
            name,
            runnable_type,
            warn,
            image,
            path,
            info,
            logo,
            logo_path,
            missing_python_packages,
            config_file_name,
            config_directory,
            script_file,
            use_virtual_environment,
            requirements_file,
            requires_tkinter,
        })
    }
}

impl Clean for ScriptConfigBundle {
    fn cleaned(&self) -> Self {
        let mut b = self.clone();
        b.logo = None;
        b.warn = None;
        b.missing_python_packages = None;
        b
    }
}

impl Runnable for ScriptConfigBundle {
    fn get_config_file_name(&self) -> &str {
        &self.config_file_name
    }

    fn get_requirements_file(&self) -> &Option<String> {
        &self.requirements_file
    }

    fn use_virtual_environment(&self) -> bool {
        self.use_virtual_environment
    }

    fn get_missing_packages(&self, window: &Window) -> Vec<String> {
        if self.use_virtual_environment() {
            return Vec::new();
        }

        let python = if let Ok(path) = PYTHON_PATH.lock() {
            path.to_owned()
        } else {
            return Vec::new();
        };

        if let Some(req_file) = self.get_requirements_file() {
            let mut args: Vec<&str> = vec!["-c", "from rlbot_smh.get_missing_packages import run; run()"];

            if self.requires_tkinter {
                args.push("requires_tkinter");
            }

            let file = format!("requirements_file={}", req_file);

            args.push(&file);

            let mut command = process::Command::new(python);

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                // disable window creation
                command.creation_flags(0x08000000);
            };

            match command.args(args).stdin(Stdio::null()).output() {
                Ok(proc) => {
                    let output = from_utf8(proc.stdout.as_slice()).unwrap();
                    if let Ok(packages) = serde_json::from_str(output) {
                        return packages;
                    }
                }
                Err(e) => ccprintln(window, format!("Failed to calculate missing packages: {}", e)),
            }
        } else if self.requires_tkinter && !get_command_status(python, ["-c", "import tkinter"]) {
            return vec![String::from("tkinter")];
        }

        Vec::new()
    }

    fn logo(&self) -> &Option<String> {
        &self.logo
    }

    fn load_logo(&self) -> Option<String> {
        if let Some(logo_path) = &self.logo_path {
            to_base64(logo_path)
        } else {
            None
        }
    }

    fn is_rlbot_controlled(&self) -> bool {
        true
    }

    fn warn(&self) -> &Option<String> {
        &self.warn
    }

    fn missing_python_packages(&self) -> &Option<Vec<String>> {
        &self.missing_python_packages
    }
}
