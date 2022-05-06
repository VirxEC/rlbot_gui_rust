use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use imghdr::Type;
use serde::{Deserialize, Serialize};
use tini::{Error, Ini};

pub const BOT_CONFIG_MODULE_HEADER: &str = "Locations";
pub const BOT_CONFIG_DETAILS_HEADER: &str = "Details";
// pub const PYTHON_FILE_KEY: &str = "python_file";
// pub const SUPPORTS_STANDALONE: &str = "supports_standalone";
// pub const LOADOUT_GENERATOR_FILE_KEY: &str = "loadout_generator";
pub const LOGO_FILE_KEY: &str = "logo_file";
pub const LOOKS_CONFIG_KEY: &str = "looks_config";
pub const BOT_NAME_KEY: &str = "name";
// pub const SUPPORTS_EARLY_START_KEY: &str = "supports_early_start";
// pub const MAXIMUM_TICK_RATE_PREFERENCE_KEY: &str = "maximum_tick_rate_preference";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BotInfo {
    pub developer: String,
    pub description: String,
    pub fun_fact: String,
    pub github: String,
    pub language: String,
    pub tags: Vec<String>,
}

impl BotInfo {
    pub fn from_config(config: Ini) -> Self {
        let developer = config.get::<String>(BOT_CONFIG_DETAILS_HEADER, "developer").unwrap_or_default();
        let description = config.get::<String>(BOT_CONFIG_DETAILS_HEADER, "description").unwrap_or_default();
        let fun_fact = config.get::<String>(BOT_CONFIG_DETAILS_HEADER, "fun_fact").unwrap_or_default();
        let github = config.get::<String>(BOT_CONFIG_DETAILS_HEADER, "github").unwrap_or_default();
        let language = config.get::<String>(BOT_CONFIG_DETAILS_HEADER, "language").unwrap_or_default();
        let tags = config
            .get_vec::<String>(BOT_CONFIG_DETAILS_HEADER, "tags")
            .unwrap_or_default()
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BotConfigBundle {
    pub name: Option<String>,
    pub looks_path: Option<String>,
    pub path: String,
    config_file_name: String,
    pub info: BotInfo,
    pub logo: Option<String>,
    pub type_: String,
    pub skill: u8,
    pub image: String,
    pub missing_python_packages: Vec<String>,
}

/// Returns a file's extension based on the its hexidecimal representation.
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
    let mut file = fs::File::open(path).unwrap();
    let mut vec = Vec::new();
    let _ = file.read_to_end(&mut vec);

    get_file_extension(&vec).map(|extension| format!("data:image/{};base64,{}", extension, base64::encode(vec).replace("\r\n", "")))
}

impl BotConfigBundle {
    pub fn from_path(config_path: PathBuf) -> Result<Self, Error> {
        let config = Ini::from_file(config_path.to_str().unwrap())?;
        let path = config_path.to_str().unwrap().to_string();
        let config_directory = config_path.parent().unwrap().to_str().unwrap().to_string();
        let config_file_name = config_path.file_name().unwrap().to_str().unwrap().to_string();

        let name = config.get(BOT_CONFIG_MODULE_HEADER, BOT_NAME_KEY);
        let looks_path = config
            .get::<String>(BOT_CONFIG_MODULE_HEADER, LOOKS_CONFIG_KEY)
            .map(|path| format!("{}/{}", config_directory, path));

        let t_logo = config.get::<String>(BOT_CONFIG_MODULE_HEADER, LOGO_FILE_KEY).unwrap_or_else(|| String::from("logo.png"));
        let ta_logo = format!("{}/{}", config_directory, t_logo);

        let logo = if Path::new(&ta_logo).exists() { to_base64(&ta_logo) } else { None };

        let info = BotInfo::from_config(config);

        let type_ = String::from("rlbot");
        let skill = 1;
        let image = String::from("imgs/rlbot.png");
        let missing_python_packages = Vec::new();

        Ok(Self {
            name,
            looks_path,
            path,
            config_file_name,
            info,
            logo,
            type_,
            skill,
            image,
            missing_python_packages,
        })
    }

    pub fn get_config_file_name(&self) -> &str {
        &self.config_file_name
    }

    pub fn is_valid_bot_config(&self) -> bool {
        if self.looks_path.is_none() || self.name.is_none() {
            return false;
        }

        true
    }
}
