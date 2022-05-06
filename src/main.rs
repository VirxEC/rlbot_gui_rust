#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod rlbot;

use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::create_dir_all,
    path::{Path, PathBuf},
    process,
    str::FromStr,
};

use lazy_static::{initialize, lazy_static};
use rlbot::agents::runnable::Runnable;
use rlbot::parsing::{
    agent_config_parser::BotLooksConfig,
    bot_config_bundle::{BotConfigBundle, ScriptConfigBundle},
    directory_scanner::scan_directory_for_script_configs,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use tini::Ini;

use crate::rlbot::parsing::directory_scanner::scan_directory_for_bot_configs;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotFolder {
    pub visible: bool,
}

impl fmt::Display for BotFolder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl FromStr for BotFolder {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotFolderSettings {
    files: HashMap<String, BotFolder>,
    folders: HashMap<String, BotFolder>,
}

impl BotFolderSettings {
    fn new() -> Self {
        let conf = Ini::from_file(&*CONFIG_PATH.lock().unwrap()).unwrap();
        let files = serde_json::from_str(&*conf.get::<String>("bot_folder_settings", "files").unwrap_or_else(|| "[]".to_string())).unwrap_or_default();

        let folders = serde_json::from_str(&*conf.get::<String>("bot_folder_settings", "folders").unwrap_or_else(|| "[]".to_string())).unwrap_or_default();

        Self { files, folders }
    }

    fn update_config(&mut self, bfs: BotFolderSettings) {
        self.files = bfs.files;
        self.folders = bfs.folders;

        let path = CONFIG_PATH.lock().unwrap();
        let conf = Ini::from_file(&*path)
            .unwrap()
            .section("bot_folder_settings")
            .item("files", serde_json::to_string(&self.files).unwrap())
            .item("folders", serde_json::to_string(&self.folders).unwrap());
        conf.to_file(&*path).unwrap();
    }

    fn add_folder(&mut self, path: String) {
        self.folders.insert(path, BotFolder { visible: true });
        self.update_config(self.clone());
    }
}

lazy_static! {
    static ref CONFIG_PATH: Mutex<String> = {
        let path = match env::consts::FAMILY {
            "windows" => Path::new("%LOCALAPPDATA%\\RLBotGUIX\\config.ini").to_path_buf(),
            "unix" => Path::new(&env::var_os("HOME").unwrap()).join(".config/rlbotgui/config.ini"),
            _ => unreachable!("Unsupported OS"),
        };

        println!("Config path: {}", path.to_str().unwrap());

        if !path.exists() {
            create_dir_all(path.parent().unwrap()).unwrap();
            let conf = Ini::new().section("bot_folder_settings").item("files", "[]").item("folders", "[]");

            conf.to_file(&path).unwrap();
        }

        Mutex::new(path.to_str().unwrap().to_string())
    };
}

lazy_static! {
    static ref BOT_FOLDER_SETTINGS: Mutex<BotFolderSettings> = Mutex::new(BotFolderSettings::new());
}

#[tauri::command]
async fn save_folder_settings(bot_folder_settings: BotFolderSettings) {
    BOT_FOLDER_SETTINGS.lock().unwrap().update_config(bot_folder_settings);
}

#[tauri::command]
async fn get_folder_settings() -> BotFolderSettings {
    BOT_FOLDER_SETTINGS.lock().unwrap().clone()
}

fn filter_hidden_bundles<T: Runnable + Clone>(bundles: HashSet<T>) -> Vec<T> {
    bundles.iter().filter(|b| !b.get_config_file_name().starts_with('_')).cloned().collect()
}

fn get_bots_from_directory(path: &str) -> Vec<BotConfigBundle> {
    filter_hidden_bundles(scan_directory_for_bot_configs(path))
}

#[tauri::command]
async fn scan_for_bots() -> Vec<BotConfigBundle> {
    let bfs = BOT_FOLDER_SETTINGS.lock().unwrap();
    let mut bots = Vec::with_capacity(bfs.folders.len() + bfs.files.len());

    for (path, props) in bfs.folders.iter() {
        if props.visible {
            bots.extend(get_bots_from_directory(&*path));
        }
    }

    for (path, props) in bfs.files.iter() {
        if props.visible {
            bots.extend(BotConfigBundle::from_path(PathBuf::from(path)));
        }
    }

    bots
}

fn get_scripts_from_directory(path: &str) -> Vec<ScriptConfigBundle> {
    filter_hidden_bundles(scan_directory_for_script_configs(path))
}

#[tauri::command]
async fn scan_for_scripts() -> Vec<ScriptConfigBundle> {
    let bfs = BOT_FOLDER_SETTINGS.lock().unwrap();
    let mut scripts = Vec::with_capacity(bfs.folders.len() + bfs.files.len());

    for (path, props) in bfs.folders.iter() {
        if props.visible {
            scripts.extend(get_scripts_from_directory(&*path));
        }
    }

    for (path, props) in bfs.files.iter() {
        if props.visible {
            scripts.extend(ScriptConfigBundle::from_path(PathBuf::from(path)));
        }
    }

    scripts
}

use native_dialog::FileDialog;

#[tauri::command]
async fn pick_bot_folder() {
    let path = match FileDialog::new().show_open_single_dir().unwrap() {
        Some(path) => path,
        None => return,
    };

    BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(path.to_str().unwrap().to_string());
}

#[tauri::command]
async fn show_path_in_explorer(path: String) {
    let command = match env::consts::FAMILY {
        "windows" => "explorer.exe",
        "unix" => "xdg-open",
        _ => unreachable!("Unsupported OS"),
    };

    let ppath = Path::new(&*path);
    let path = if ppath.is_file() { ppath.parent().unwrap().to_str().unwrap() } else { &*path };

    process::Command::new(command).arg(path).spawn().unwrap();
}

#[tauri::command]
async fn get_looks(path: String) -> Option<BotLooksConfig> {
    match BotLooksConfig::from_path(&*path) {
        Ok(looks) => Some(looks),
        Err(_) => None,
    }
}

#[tauri::command]
async fn save_looks(path: String, config: BotLooksConfig) {
    config.save_to_path(&*path);
}

fn main() {
    initialize(&CONFIG_PATH);
    initialize(&BOT_FOLDER_SETTINGS);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_folder_settings,
            save_folder_settings,
            pick_bot_folder,
            show_path_in_explorer,
            scan_for_bots,
            get_looks,
            save_looks,
            scan_for_scripts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
