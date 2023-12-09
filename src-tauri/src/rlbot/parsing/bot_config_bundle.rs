#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::{
    bot_management::cfg_helper::{load_cfg, load_cfg_sync, Error},
    ccprintln, get_command_status,
    rlbot::agents::{base_script::SCRIPT_FILE_KEY, runnable::Runnable},
};
use base64::{prelude::BASE64_STANDARD, Engine};
use configparser::ini::Ini;
use imghdr::Type;
use serde::{Deserialize, Serialize};
use std::{
    borrow::ToOwned,
    ffi::OsStr,
    fs,
    io::Read,
    path::Path,
    process::{self, Stdio},
    str::from_utf8,
};
use tauri::Window;
use thiserror::Error;

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
    let Ok(file) = &mut fs::File::open(path) else {
        return None;
    };

    let mut vec = Vec::new();
    file.read_to_end(&mut vec).ok()?;

    let encoded_string = BASE64_STANDARD.encode(&vec).replace("\r\n", "");
    get_file_extension(&vec).map(|extension| format!("data:image/{extension};base64,{encoded_string}"))
}

#[derive(Debug, Error)]
pub enum RLBotCfgParseError {
    #[error(transparent)]
    CfgLoad(#[from] Error),
    #[error("No name found in config file {0}")]
    NoName(String),
    #[error("No looks config found in config file {0}")]
    NoLooksConfig(String),
    #[error("No python file found in config file {0}")]
    NoPythonFile(String),
    #[error("No script file found in config file {0}")]
    NoScriptFile(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct BotConfigBundle {
    pub name: String,
    pub skill: Option<f32>,
    pub looks_path: String,
    pub path: String,
    config_file_name: String,
    config_directory: String,
    pub info: Option<DevInfo>,
    pub logo_path: String,
    pub logo: Option<String>,
    pub runnable_type: String,
    pub warn: Option<String>,
    pub image: String,
    supports_standalone: bool,
    use_virtual_environment: bool,
    requirements_file: Option<String>,
    requires_tkinter: bool,
    pub missing_python_packages: Option<Vec<String>>,
    pub python_path: String,
}

impl BotConfigBundle {
    pub fn new_human() -> Self {
        Self {
            name: "Human".to_owned(),
            runnable_type: "human".to_owned(),
            image: "imgs/human.png".to_owned(),
            ..Default::default()
        }
    }

    pub fn new_psyonix(skill: f32) -> Self {
        let name = if (skill - 1.).abs() < f32::EPSILON {
            "Psyonix Allstar"
        } else if (skill - 0.5).abs() < f32::EPSILON {
            "Psyonix Pro"
        } else {
            "Psyonix Rookie"
        }
        .to_owned();

        Self {
            name,
            skill: Some(skill),
            runnable_type: "psyonix".to_owned(),
            image: "imgs/psyonix.png".to_owned(),
            ..Default::default()
        }
    }

    pub async fn minimal_from_path<T: AsRef<Path> + Send>(config_path: T) -> Result<Self, RLBotCfgParseError> {
        let config_path = config_path.as_ref();
        Self::minimal_from_conf(config_path, &load_cfg(config_path).await?)
    }

    pub fn minimal_from_path_sync(config_path: &Path) -> Result<Self, RLBotCfgParseError> {
        Self::minimal_from_conf(config_path, &load_cfg_sync(config_path)?)
    }

    fn minimal_from_conf(config_path: &Path, conf: &Ini) -> Result<Self, RLBotCfgParseError> {
        let path = config_path.to_string_lossy().to_string();
        let config_path_str = config_path.display().to_string();
        // the follow unwrap calls will probably never fail because the config file was loaded successfully, already
        let config_file_name = config_path.file_name().unwrap().to_string_lossy().to_string();
        let config_directory = config_path.parent().unwrap().to_string_lossy().to_string();

        let python_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, PYTHON_FILE_KEY)
            .map(|path| format!("{config_directory}/{path}"))
            .ok_or_else(|| RLBotCfgParseError::NoPythonFile(config_path_str.clone()))?;

        if !Path::new(&python_path).exists() {
            return Err(RLBotCfgParseError::NoPythonFile(config_path_str));
        }

        let name = conf
            .get(BOT_CONFIG_MODULE_HEADER, NAME_KEY)
            .ok_or_else(|| RLBotCfgParseError::NoName(config_path_str.clone()))?;

        let looks_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, LOOKS_CONFIG_KEY)
            .map(|path| format!("{config_directory}/{path}"))
            .ok_or_else(|| RLBotCfgParseError::NoLooksConfig(config_path_str.clone()))?;
        let supports_standalone = conf
            .getboolcoerce(BOT_CONFIG_MODULE_HEADER, SUPPORTS_STANDALONE)
            .unwrap_or_default()
            .unwrap_or_default();
        let use_virtual_environment = conf
            .getboolcoerce(BOT_CONFIG_MODULE_HEADER, USE_VIRTUAL_ENVIRONMENT_KEY)
            .unwrap_or_default()
            .unwrap_or_default();
        let requirements_file = conf
            .get(BOT_CONFIG_MODULE_HEADER, REQUIREMENTS_FILE_KEY)
            .map(|path| format!("{config_directory}/{path}"));
        let requires_tkinter = conf
            .getboolcoerce(BOT_CONFIG_MODULE_HEADER, REQUIRES_TKINTER)
            .unwrap_or_default()
            .unwrap_or_default();

        if !Path::new(&looks_path).exists() {
            return Err(RLBotCfgParseError::NoLooksConfig(config_path_str));
        }

        let relative_logo_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, LOGO_FILE_KEY)
            .unwrap_or_else(|| String::from("logo.png"));
        let logo_path = format!("{config_directory}/{relative_logo_path}");

        let logo = None;

        let info = Some(DevInfo::from_config(conf));

        let runnable_type = String::from("rlbot");
        let warn = None;
        let image = String::from("imgs/rlbot.png");
        let missing_python_packages = None;

        Ok(Self {
            name,
            skill: None,
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

    pub fn name_from_path(config_path: &Path) -> Result<(String, String), RLBotCfgParseError> {
        let config_path_str = config_path.display().to_string();
        let conf = load_cfg_sync(config_path)?;

        let Some(name) = conf.get(BOT_CONFIG_MODULE_HEADER, NAME_KEY) else {
            return Err(RLBotCfgParseError::NoName(config_path_str));
        };

        let path = config_path.to_string_lossy().to_string();

        let config_directory = config_path.parent().unwrap();

        let looks_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, LOOKS_CONFIG_KEY)
            .map(|path| format!("{}/{path}", config_directory.display()));

        let valid_looks = looks_path.as_ref().map_or(false, |path| Path::new(path).exists());

        if !valid_looks {
            return Err(RLBotCfgParseError::NoLooksConfig(config_path_str));
        }

        let python_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, PYTHON_FILE_KEY)
            .map(|path| format!("{}/{path}", config_directory.display()));

        let valid_path = python_path.as_ref().map_or(false, |path| Path::new(path).exists());

        if !valid_path {
            return Err(RLBotCfgParseError::NoPythonFile(config_path_str));
        }

        Ok((name, path))
    }
}

impl Runnable for BotConfigBundle {
    fn get_config_file_name(&self) -> &str {
        &self.config_file_name
    }

    fn get_requirements_file(&self) -> &Option<String> {
        &self.requirements_file
    }

    fn use_virtual_environment(&self) -> bool {
        self.supports_standalone && self.use_virtual_environment
    }

    fn get_missing_packages<S: AsRef<OsStr>>(&self, window: &Window, python: S) -> Vec<String> {
        if self.use_virtual_environment() {
            return Vec::new();
        }

        let Some(req_file) = self.get_requirements_file() else {
            return if self.requires_tkinter && !get_command_status(python, ["-c", "import tkinter"]) {
                vec![String::from("tkinter")]
            } else {
                Vec::new()
            };
        };

        let mut args: Vec<&str> = vec!["-c", "from rlbot_smh.get_missing_packages import run; run()"];

        if self.requires_tkinter {
            args.push("requires_tkinter");
        }

        let file = format!("requirements_file={req_file}");

        args.push(&file);

        let mut command = process::Command::new(python);

        #[cfg(windows)]
        {
            // disable window creation
            command.creation_flags(0x0800_0000);
        };

        match command.args(args).stdin(Stdio::null()).output() {
            Ok(proc) => {
                let output = from_utf8(proc.stdout.as_slice()).unwrap();
                serde_json::from_str(output).unwrap_or_default()
            }
            Err(e) => {
                ccprintln(window, format!("Failed to calculate missing packages: {e}"));
                Vec::new()
            }
        }
    }

    fn logo(&self) -> &Option<String> {
        &self.logo
    }

    fn load_logo(&self) -> Option<String> {
        to_base64(&self.logo_path)
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

    fn may_require_python_packages(&self) -> bool {
        self.info
            .as_ref()
            .map(|info| info.language.to_lowercase().contains("python"))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct ScriptConfigBundle {
    pub name: String,
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
    pub async fn minimal_from_path<T: AsRef<Path> + Send>(config_path: T) -> Result<Self, RLBotCfgParseError> {
        let config_path = config_path.as_ref();
        let conf = load_cfg(config_path).await?;

        let config_path_str = config_path.display().to_string();
        // the follow unwrap calls will probably never fail because the config file was loaded successfully, already
        let config_file_name = config_path.file_name().unwrap().to_string_lossy().to_string();
        let config_directory = config_path.parent().unwrap().to_string_lossy().to_string();

        let script_file = conf
            .get(BOT_CONFIG_MODULE_HEADER, SCRIPT_FILE_KEY)
            .map(|path| format!("{config_directory}/{path}"))
            .ok_or_else(|| RLBotCfgParseError::NoScriptFile(config_path_str.clone()))?;

        if !Path::new(&script_file).exists() {
            return Err(RLBotCfgParseError::NoScriptFile(config_path_str));
        }

        let name = conf
            .get(BOT_CONFIG_MODULE_HEADER, NAME_KEY)
            .ok_or_else(|| RLBotCfgParseError::NoName(config_path_str.clone()))?;
        let runnable_type = String::from("script");
        let warn = None;
        let image = String::from("imgs/rlbot.png");
        let path = config_path.to_string_lossy().to_string();
        let use_virtual_environment = conf
            .getboolcoerce(BOT_CONFIG_MODULE_HEADER, USE_VIRTUAL_ENVIRONMENT_KEY)
            .unwrap_or(None)
            .unwrap_or_default();
        let requirements_file = conf
            .get(BOT_CONFIG_MODULE_HEADER, REQUIREMENTS_FILE_KEY)
            .map(|path| format!("{config_directory}/{path}"));
        let requires_tkinter = conf
            .getboolcoerce(BOT_CONFIG_MODULE_HEADER, REQUIRES_TKINTER)
            .unwrap_or(None)
            .unwrap_or_default();

        let relative_logo_path = conf
            .get(BOT_CONFIG_MODULE_HEADER, LOGO_FILE_KEY)
            .unwrap_or_else(|| String::from("logo.png"));
        let absolute_logo_path = format!("{config_directory}/{relative_logo_path}");
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

    fn get_missing_packages<S: AsRef<OsStr>>(&self, window: &Window, python: S) -> Vec<String> {
        if self.use_virtual_environment() {
            return Vec::new();
        }

        let Some(req_file) = self.get_requirements_file() else {
            return if self.requires_tkinter && !get_command_status(python, ["-c", "import tkinter"]) {
                vec![String::from("tkinter")]
            } else {
                Vec::new()
            };
        };

        let mut args: Vec<&str> = vec!["-c", "from rlbot_smh.get_missing_packages import run; run()"];

        if self.requires_tkinter {
            args.push("requires_tkinter");
        }

        let file = format!("requirements_file={req_file}");

        args.push(&file);

        let mut command = process::Command::new(python);

        #[cfg(windows)]
        {
            // disable window creation
            command.creation_flags(0x0800_0000);
        };

        match command.args(args).stdin(Stdio::null()).output() {
            Ok(proc) => {
                let output = from_utf8(proc.stdout.as_slice()).unwrap();
                serde_json::from_str(output).unwrap_or_default()
            }
            Err(e) => {
                ccprintln(window, format!("Failed to calculate missing packages: {e}"));
                Vec::new()
            }
        }
    }

    fn logo(&self) -> &Option<String> {
        &self.logo
    }

    fn load_logo(&self) -> Option<String> {
        let Some(logo_path) = &self.logo_path else {
            return None;
        };

        to_base64(logo_path)
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

    fn may_require_python_packages(&self) -> bool {
        true
    }
}
