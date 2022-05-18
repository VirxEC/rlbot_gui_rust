#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod custom_maps;
mod rlbot;

use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, create_dir_all, read_to_string},
    io::Cursor,
    path::Path,
    process::{self, Stdio},
    str::FromStr,
};

use glob::glob;

use custom_maps::find_all_custom_maps;
use lazy_static::{initialize, lazy_static};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rlbot::parsing::{
    agent_config_parser::BotLooksConfig,
    bot_config_bundle::{BotConfigBundle, Clean, ScriptConfigBundle, BOT_CONFIG_MODULE_HEADER, PYTHON_FILE_KEY},
    directory_scanner::scan_directory_for_script_configs,
};
use rlbot::{agents::runnable::Runnable, parsing::match_settings_config_parser::*};
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use tini::Ini;

use rlbot::parsing::directory_scanner::scan_directory_for_bot_configs;
use tauri::{SystemTray, SystemTrayMenu};

const CREATED_BOTS_FOLDER: &str = "MyBots";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotFolder {
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
pub struct BotFolderSettings {
    pub files: HashMap<String, BotFolder>,
    pub folders: HashMap<String, BotFolder>,
}

impl BotFolderSettings {
    fn from_path(path: &String) -> Self {
        let conf = Ini::from_file(path).unwrap();
        let files = serde_json::from_str(&*conf.get::<String>("bot_folder_settings", "files").unwrap_or_else(|| "[]".to_string())).unwrap_or_default();

        let folders = serde_json::from_str(&*conf.get::<String>("bot_folder_settings", "folders").unwrap_or_else(|| "[]".to_string())).unwrap_or_default();

        Self { files, folders }
    }

    fn update_config(&mut self, bfs: Self) {
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

    fn add_file(&mut self, path: String) {
        self.files.insert(path, BotFolder { visible: true });
        self.update_config(self.clone());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MutatorSettings {
    pub match_length: String,
    pub max_score: String,
    pub overtime: String,
    pub series_length: String,
    pub game_speed: String,
    pub ball_max_speed: String,
    pub ball_type: String,
    pub ball_weight: String,
    pub ball_size: String,
    pub ball_bounciness: String,
    pub boost_amount: String,
    pub rumble: String,
    pub boost_strength: String,
    pub gravity: String,
    pub demolish: String,
    pub respawn_time: String,
}

impl MutatorSettings {
    fn from_path(path: &String) -> Self {
        let conf = Ini::from_file(path).unwrap();

        let match_length = conf.get::<String>("mutator_settings", "match_length").unwrap_or_else(|| MATCH_LENGTH_TYPES[0].to_string());
        let max_score = conf.get::<String>("mutator_settings", "max_score").unwrap_or_else(|| MAX_SCORE_TYPES[0].to_string());
        let overtime = conf.get::<String>("mutator_settings", "overtime").unwrap_or_else(|| OVERTIME_MUTATOR_TYPES[0].to_string());
        let series_length = conf
            .get::<String>("mutator_settings", "series_length")
            .unwrap_or_else(|| SERIES_LENGTH_MUTATOR_TYPES[0].to_string());
        let game_speed = conf
            .get::<String>("mutator_settings", "game_speed")
            .unwrap_or_else(|| GAME_SPEED_MUTATOR_TYPES[0].to_string());
        let ball_max_speed = conf
            .get::<String>("mutator_settings", "ball_max_speed")
            .unwrap_or_else(|| BALL_MAX_SPEED_MUTATOR_TYPES[0].to_string());
        let ball_type = conf
            .get::<String>("mutator_settings", "ball_type")
            .unwrap_or_else(|| BALL_TYPE_MUTATOR_TYPES[0].to_string());
        let ball_weight = conf
            .get::<String>("mutator_settings", "ball_weight")
            .unwrap_or_else(|| BALL_WEIGHT_MUTATOR_TYPES[0].to_string());
        let ball_size = conf
            .get::<String>("mutator_settings", "ball_size")
            .unwrap_or_else(|| BALL_SIZE_MUTATOR_TYPES[0].to_string());
        let ball_bounciness = conf
            .get::<String>("mutator_settings", "ball_bounciness")
            .unwrap_or_else(|| BALL_BOUNCINESS_MUTATOR_TYPES[0].to_string());
        let boost_amount = conf
            .get::<String>("mutator_settings", "boost_amount")
            .unwrap_or_else(|| BOOST_AMOUNT_MUTATOR_TYPES[0].to_string());
        let rumble = conf.get::<String>("mutator_settings", "rumble").unwrap_or_else(|| RUMBLE_MUTATOR_TYPES[0].to_string());
        let boost_strength = conf
            .get::<String>("mutator_settings", "boost_strength")
            .unwrap_or_else(|| BOOST_STRENGTH_MUTATOR_TYPES[0].to_string());
        let gravity = conf.get::<String>("mutator_settings", "gravity").unwrap_or_else(|| GRAVITY_MUTATOR_TYPES[0].to_string());
        let demolish = conf.get::<String>("mutator_settings", "demolish").unwrap_or_else(|| DEMOLISH_MUTATOR_TYPES[0].to_string());
        let respawn_time = conf
            .get::<String>("mutator_settings", "respawn_time")
            .unwrap_or_else(|| RESPAWN_TIME_MUTATOR_TYPES[0].to_string());

        Self {
            match_length,
            max_score,
            overtime,
            series_length,
            game_speed,
            ball_max_speed,
            ball_type,
            ball_weight,
            ball_size,
            ball_bounciness,
            boost_amount,
            rumble,
            boost_strength,
            gravity,
            demolish,
            respawn_time,
        }
    }

    fn update_config(&mut self, ms: Self) {
        self.match_length = ms.match_length;
        self.max_score = ms.max_score;
        self.overtime = ms.overtime;
        self.series_length = ms.series_length;
        self.game_speed = ms.game_speed;
        self.ball_max_speed = ms.ball_max_speed;
        self.ball_type = ms.ball_type;
        self.ball_weight = ms.ball_weight;
        self.ball_size = ms.ball_size;
        self.ball_bounciness = ms.ball_bounciness;
        self.boost_amount = ms.boost_amount;
        self.rumble = ms.rumble;
        self.boost_strength = ms.boost_strength;
        self.gravity = ms.gravity;
        self.demolish = ms.demolish;
        self.respawn_time = ms.respawn_time;

        let path = CONFIG_PATH.lock().unwrap();
        let conf = Ini::from_file(&*path)
            .unwrap()
            .section("mutator_settings")
            .item("match_length", &self.match_length)
            .item("max_score", &self.max_score)
            .item("overtime", &self.overtime)
            .item("series_length", &self.series_length)
            .item("game_speed", &self.game_speed)
            .item("ball_max_speed", &self.ball_max_speed)
            .item("ball_type", &self.ball_type)
            .item("ball_weight", &self.ball_weight)
            .item("ball_size", &self.ball_size)
            .item("ball_bounciness", &self.ball_bounciness)
            .item("boost_amount", &self.boost_amount)
            .item("rumble", &self.rumble)
            .item("boost_strength", &self.boost_strength)
            .item("gravity", &self.gravity)
            .item("demolish", &self.demolish)
            .item("respawn_time", &self.respawn_time);
        conf.to_file(&*path).unwrap();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MatchSettings {
    pub map: String,
    pub game_mode: String,
    pub match_behavior: String,
    pub skip_replays: bool,
    pub instant_start: bool,
    pub enable_lockstep: bool,
    pub randomize_map: bool,
    pub enable_rendering: bool,
    pub enable_state_setting: bool,
    pub auto_save_replay: bool,
    pub scripts: Vec<ScriptConfigBundle>,
    pub mutators: MutatorSettings,
}

impl MatchSettings {
    fn from_path(path: &String) -> Self {
        let conf = Ini::from_file(path).unwrap();

        let map = conf.get::<String>("match_settings", "map").unwrap_or_else(|| MAP_TYPES[0].to_string());
        let game_mode = conf.get::<String>("match_settings", "game_mode").unwrap_or_else(|| GAME_MODES[0].to_string());
        let match_behavior = conf
            .get::<String>("match_settings", "match_behavior")
            .unwrap_or_else(|| EXISTING_MATCH_BEHAVIOR_TYPES[0].to_string());
        let skip_replays = conf.get::<bool>("match_settings", "skip_replays").unwrap_or(false);
        let instant_start = conf.get::<bool>("match_settings", "instant_start").unwrap_or(false);
        let enable_lockstep = conf.get::<bool>("match_settings", "enable_lockstep").unwrap_or(false);
        let randomize_map = conf.get::<bool>("match_settings", "randomize_map").unwrap_or(false);
        let enable_rendering = conf.get::<bool>("match_settings", "enable_rendering").unwrap_or(false);
        let enable_state_setting = conf.get::<bool>("match_settings", "enable_state_setting").unwrap_or(true);
        let auto_save_replay = conf.get::<bool>("match_settings", "auto_save_replay").unwrap_or(false);
        let scripts = conf.get::<String>("match_settings", "scripts").unwrap_or_else(|| "[]".to_string());

        Self {
            map,
            game_mode,
            match_behavior,
            skip_replays,
            instant_start,
            enable_lockstep,
            randomize_map,
            enable_rendering,
            enable_state_setting,
            auto_save_replay,
            scripts: serde_json::from_str(&scripts).unwrap_or_default(),
            mutators: MutatorSettings::from_path(path),
        }
    }

    fn update_config(&mut self, ms: Self) {
        self.map = ms.map;
        self.game_mode = ms.game_mode;
        self.match_behavior = ms.match_behavior;
        self.skip_replays = ms.skip_replays;
        self.instant_start = ms.instant_start;
        self.enable_lockstep = ms.enable_lockstep;
        self.randomize_map = ms.randomize_map;
        self.enable_rendering = ms.enable_rendering;
        self.enable_state_setting = ms.enable_state_setting;
        self.auto_save_replay = ms.auto_save_replay;
        self.scripts = ms.scripts;

        self.mutators.update_config(ms.mutators);

        let path = CONFIG_PATH.lock().unwrap();
        let conf = Ini::from_file(&*path)
            .unwrap()
            .section("match_settings")
            .item("map", &self.map)
            .item("game_mode", &self.game_mode)
            .item("match_behavior", &self.match_behavior)
            .item("skip_replays", self.skip_replays)
            .item("instant_start", self.instant_start)
            .item("enable_lockstep", self.enable_lockstep)
            .item("randomize_map", self.randomize_map)
            .item("enable_rendering", self.enable_rendering)
            .item("enable_state_setting", self.enable_state_setting)
            .item("auto_save_replay", self.auto_save_replay)
            .item("scripts", serde_json::to_string(&self.scripts).unwrap_or_default());
        conf.to_file(&*path).unwrap();
    }

    fn cleaned_scripts(&self) -> Self {
        let mut new = self.clone();
        new.scripts = clean(new.scripts);
        new
    }

    fn with_logos(&self) -> Self {
        let mut new = self.clone();
        new.scripts = fetch_logos(new.scripts);
        new
    }
}

fn auto_detect_python() -> String {
    if cfg!(target_os = "windows") {
        match Path::new(&env::var_os("LOCALAPPDATA").unwrap()).join("RLBotGUIX\\Python37\\python.exe") {
            path if path.exists() => path.to_str().unwrap().to_string(),
            _ => match Path::new(&env::var_os("LOCALAPPDATA").unwrap()).join("RLBotGUIX\\venv\\python.exe") {
                path if path.exists() => path.to_str().unwrap().to_string(),
                _ => "python3.7".to_string(),
            },
        }
    } else if cfg!(target_os = "macos") {
        "python3.7".to_string()
    } else {
        match Path::new(&env::var_os("HOME").unwrap()).join(".RLBotGUI/env/bin/python") {
            path if path.exists() => path.to_str().unwrap().to_string(),
            _ => "python3.7".to_string(),
        }
    }
}

lazy_static! {
    static ref CONFIG_PATH: Mutex<String> = {
        let path = if cfg!(target_os = "windows") {
            Path::new(&env::var_os("LOCALAPPDATA").unwrap()).join("RLBotGUIX\\config.ini")
        } else if cfg!(target_os = "macos") {
            Path::new(&env::var_os("HOME").unwrap()).join("Library/Application Support/rlbotgui/config.ini")
        } else {
            Path::new(&env::var_os("HOME").unwrap()).join(".config/rlbotgui/config.ini")
        };

        println!("Config path: {}", path.to_str().unwrap());

        if !path.exists() {
            create_dir_all(path.parent().unwrap()).unwrap();
            let conf = Ini::new()
                .section("bot_folder_settings")
                .item("files", "{}")
                .item("folders", "{}")
                .section("match_settings")
                .item("map", MAP_TYPES[0].to_string())
                .item("game_mode", GAME_MODES[0].to_string())
                .item("match_behavior", EXISTING_MATCH_BEHAVIOR_TYPES[0].to_string())
                .item("skip_replays", false)
                .item("instant_start", false)
                .item("enable_lockstep", false)
                .item("randomize_map", false)
                .item("enable_rendering", false)
                .item("enable_state_setting", true)
                .item("auto_save_replay", false)
                .item("scripts", "[]")
                .section("mutator_settings")
                .item("match_length", MATCH_LENGTH_TYPES[0].to_string())
                .item("max_score", MAX_SCORE_TYPES[0].to_string())
                .item("overtime", OVERTIME_MUTATOR_TYPES[0].to_string())
                .item("series_length", SERIES_LENGTH_MUTATOR_TYPES[0].to_string())
                .item("game_speed", GAME_SPEED_MUTATOR_TYPES[0].to_string())
                .item("ball_max_speed", BALL_MAX_SPEED_MUTATOR_TYPES[0].to_string())
                .item("ball_type", BALL_TYPE_MUTATOR_TYPES[0].to_string())
                .item("ball_weight", BALL_WEIGHT_MUTATOR_TYPES[0].to_string())
                .item("ball_size", BALL_SIZE_MUTATOR_TYPES[0].to_string())
                .item("ball_bounciness", BALL_BOUNCINESS_MUTATOR_TYPES[0].to_string())
                .item("boost_amount", BOOST_AMOUNT_MUTATOR_TYPES[0].to_string())
                .item("rumble", RUMBLE_MUTATOR_TYPES[0].to_string())
                .item("boost_strength", BOOST_STRENGTH_MUTATOR_TYPES[0].to_string())
                .item("gravity", GRAVITY_MUTATOR_TYPES[0].to_string())
                .item("demolish", DEMOLISH_MUTATOR_TYPES[0].to_string())
                .item("respawn_time", RESPAWN_TIME_MUTATOR_TYPES[0].to_string())
                .section("python_config")
                .item("path", auto_detect_python());

            conf.to_file(&path).unwrap();
        }

        Mutex::new(path.to_str().unwrap().to_string())
    };
}

lazy_static! {
    static ref BOT_FOLDER_SETTINGS: Mutex<BotFolderSettings> = Mutex::new(BotFolderSettings::from_path(&*CONFIG_PATH.lock().unwrap()));
    static ref MATCH_SETTINGS: Mutex<MatchSettings> = Mutex::new(MatchSettings::from_path(&*CONFIG_PATH.lock().unwrap()));
    static ref PYTHON_PATH: Mutex<String> = Mutex::new({
        let config = Ini::from_file(&*CONFIG_PATH.lock().unwrap()).unwrap();
        config.get("python_config", "path").unwrap_or_else(auto_detect_python)
    });
}

#[tauri::command]
async fn save_folder_settings(bot_folder_settings: BotFolderSettings) {
    BOT_FOLDER_SETTINGS.lock().unwrap().update_config(bot_folder_settings)
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

fn scan_for_bots_r() -> Vec<BotConfigBundle> {
    let bfs = BOT_FOLDER_SETTINGS.lock().unwrap();
    let mut bots = Vec::new();

    for (path, props) in bfs.folders.iter() {
        if props.visible {
            bots.extend(get_bots_from_directory(&*path));
        }
    }

    for (path, props) in bfs.files.iter() {
        if props.visible {
            let bundle = BotConfigBundle::from_path(Path::new(path));
            if let Ok(bundle) = bundle {
                bots.push(bundle);
            }
        }
    }

    bots
}

#[tauri::command]
async fn scan_for_bots() -> Vec<BotConfigBundle> {
    scan_for_bots_r()
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
            let bundle = ScriptConfigBundle::from_path(Path::new(path));
            if let Ok(bundle) = bundle {
                scripts.push(bundle);
            }
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
async fn pick_bot_config() {
    let path = match FileDialog::new().add_filter("Bot Cfg File", &["cfg"]).show_open_single_file().unwrap() {
        Some(path) => path,
        None => return,
    };

    BOT_FOLDER_SETTINGS.lock().unwrap().add_file(path.to_str().unwrap().to_string());
}

#[tauri::command]
async fn show_path_in_explorer(path: String) {
    let command = if cfg!(target_os = "windows") {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
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

#[tauri::command]
async fn get_match_options() -> MatchOptions {
    let mut mo = MatchOptions::new();
    mo.map_types.extend(find_all_custom_maps(&BOT_FOLDER_SETTINGS.lock().unwrap().folders));
    mo
}

#[tauri::command]
async fn get_match_settings() -> MatchSettings {
    MATCH_SETTINGS.lock().unwrap().clone().with_logos()
}

#[tauri::command]
async fn save_match_settings(settings: MatchSettings) {
    MATCH_SETTINGS.lock().unwrap().update_config(settings.cleaned_scripts());
}

fn fetch_logos<T: Clean>(items: Vec<T>) -> Vec<T> {
    items.iter().map(|b| b.with_logo()).collect()
}

#[tauri::command]
async fn get_team_settings() -> HashMap<String, Vec<BotConfigBundle>> {
    let config = Ini::from_file(&*CONFIG_PATH.lock().unwrap()).unwrap();
    let blue_team = serde_json::from_str(
        &*config
            .get::<String>("team_settings", "blue_team")
            .unwrap_or_else(|| "[{\"name\": \"Human\", \"type_\": \"human\", \"image\": \"imgs/human.png\"}]".to_string()),
    )
    .unwrap();
    let orange_team = serde_json::from_str(&*config.get::<String>("team_settings", "orange_team").unwrap_or_else(|| "[]".to_string())).unwrap();

    let mut bots = HashMap::new();
    bots.insert("blue_team".to_string(), fetch_logos(blue_team));
    bots.insert("orange_team".to_string(), fetch_logos(orange_team));

    bots
}

fn clean<T: Clean>(items: Vec<T>) -> Vec<T> {
    items.iter().map(|i| i.cleaned()).collect()
}

#[tauri::command]
async fn save_team_settings(blue_team: Vec<BotConfigBundle>, orange_team: Vec<BotConfigBundle>) {
    let config = Ini::from_file(&*CONFIG_PATH.lock().unwrap())
        .unwrap()
        .section("team_settings")
        .item("blue_team", serde_json::to_string(&clean(blue_team)).unwrap())
        .item("orange_team", serde_json::to_string(&clean(orange_team)).unwrap());
    config.to_file(&*CONFIG_PATH.lock().unwrap()).unwrap();
}

fn get_command_status(program: &str, version: Vec<&str>) -> bool {
    match process::Command::new(program).args(version).stdout(Stdio::null()).stderr(Stdio::null()).status() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

#[cfg(windows)]
fn has_chrome() -> bool {
    use registry::{Hive, Security};
    let reg_path = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe";

    for install_type in [Hive::CurrentUser, Hive::LocalMachine].iter() {
        let reg_key = match install_type.open(reg_path, Security::Read) {
            Ok(key) => key,
            Err(_) => continue,
        };

        if let Ok(chrome_path) = reg_key.value("") {
            if Path::new(&chrome_path.to_string()).is_file() {
                return true;
            }
        }
    }

    false
}

#[cfg(target_os = "macos")]
fn has_chrome() -> bool {
    get_command_status("/Applications/Google\\ Chrome.app/Contents/MacOS/Google\\ Chrome", vec!["--version"])
}

#[cfg(target_os = "linux")]
fn has_chrome() -> bool {
    // google chrome works, but many Linux users especally may prefer to use Chromium instead
    get_command_status("google-chrome", vec!["--product-version"]) || get_command_status("chromium", vec!["--product-version"])
}

#[tauri::command]
async fn get_language_support() -> HashMap<String, bool> {
    let mut lang_support = HashMap::new();

    lang_support.insert("java".to_string(), get_command_status("java", vec!["-version"]));
    lang_support.insert("node".to_string(), get_command_status("node", vec!["--version"]));
    lang_support.insert("chrome".to_string(), has_chrome());

    let python_path = PYTHON_PATH.lock().unwrap().to_string();
    let python_check = get_command_status(&*python_path, vec!["--version"]);
    lang_support.insert("python".to_string(), python_check);
    lang_support.insert("fullpython".to_string(), python_check && get_command_status(&*python_path, vec!["-c", "import tkinter"]));

    lang_support
}

#[tauri::command]
async fn get_python_path() -> String {
    PYTHON_PATH.lock().unwrap().to_string()
}

#[tauri::command]
async fn set_python_path(path: String) {
    *PYTHON_PATH.lock().unwrap() = path.clone();
    let config_path = CONFIG_PATH.lock().unwrap();
    let config = Ini::from_file(&*config_path).unwrap().section("python_config").item("path", path);
    config.to_file(&*config_path).unwrap();
}

#[tauri::command]
async fn pick_appearance_file() -> Option<String> {
    match FileDialog::new().add_filter("Appearance Cfg File", &["cfg"]).show_open_single_file() {
        Ok(path) => path.map(|path| path.to_str().unwrap().to_string()),
        Err(e) => {
            dbg!(e);
            None
        }
    }
}

#[tauri::command]
async fn get_recommendations() -> Option<HashMap<String, Vec<HashMap<String, Vec<BotConfigBundle>>>>> {
    type BotNames = Vec<String>;
    type Recommendation = HashMap<String, BotNames>;
    type AllRecommendations = HashMap<String, Vec<Recommendation>>;
    let mut json: Option<AllRecommendations> = None;

    {
        let bfs = BOT_FOLDER_SETTINGS.lock().unwrap();

        for path in bfs.folders.keys() {
            let pattern = Path::new(path).join("**/recommendations.json");

            for path2 in glob(pattern.to_str().unwrap()).unwrap().flatten() {
                let raw_json = match read_to_string(&path2) {
                    Ok(s) => s,
                    Err(_) => {
                        println!("Failed to read {}", path2.to_str().unwrap());
                        continue;
                    }
                };

                match serde_json::from_str(&raw_json) {
                    Ok(j) => json = Some(j),
                    Err(e) => {
                        println!("Failed to parse file {}: {}", path2.to_str().unwrap(), e);
                        continue;
                    }
                }
            }
        }
    }

    // this can be optimized if need, but for now it's fine
    // it loads all visible bot config bundles when we really only need name/path pairs
    // if a match is found, only that bundle could get loaded
    let mut bot_config_bundles = scan_for_bots_r();

    {
        let bfs = BOT_FOLDER_SETTINGS.lock().unwrap();
        for (path, settings) in bfs.files.iter() {
            if settings.visible {
                if let Ok(bundle) = BotConfigBundle::from_path(Path::new(path)) {
                    bot_config_bundles.push(bundle);
                }
            }
        }
    }

    json.map(|j| {
        let mut recommendations: Vec<HashMap<String, Vec<BotConfigBundle>>> = Vec::new();

        for bots in j.get("recommendations").unwrap() {
            recommendations.push(HashMap::from([(
                "bots".to_string(),
                bots.get("bots")
                    .unwrap()
                    .par_iter()
                    .filter_map(|bot_name| {
                        for bundle in &bot_config_bundles {
                            if let Some(name) = &bundle.name {
                                if name == bot_name {
                                    return Some(bundle.clone());
                                }
                            }
                        }

                        None
                    })
                    .collect(),
            )]));
        }

        HashMap::from([("recommendations".to_string(), recommendations)])
    })
}

fn get_content_folder() -> String {
    let current_folder = env::current_dir().unwrap().to_str().unwrap().to_string();

    if current_folder.contains("RLBotGUI") {
        current_folder
    } else {
        match env::var_os("LOCALAPPDATA") {
            Some(path) => Path::new(&path).join("RLBotGUIX").to_str().unwrap().to_string(),
            None => current_folder,
        }
    }
}

fn ensure_bot_directory() -> String {
    let bot_directory = get_content_folder();
    let bot_directory_path = Path::new(&bot_directory).join(CREATED_BOTS_FOLDER);

    if !bot_directory_path.exists() {
        fs::create_dir_all(&bot_directory_path).unwrap();
    }

    bot_directory
}

fn bootstrap_python_bot(bot_name: String, directory: &str) -> Result<String, String> {
    let sanitized_name = sanitize(&bot_name);
    let top_dir = Path::new(directory).join(&sanitized_name);

    if top_dir.exists() {
        return Err(format!("There is already a bot named {}, please choose a different name!", sanitized_name));
    }

    match reqwest::blocking::get("https://github.com/RLBot/RLBotPythonExample/archive/master.zip") {
        Ok(res) => {
            zip_extract::extract(Cursor::new(res.text().unwrap().as_bytes()), top_dir.as_path(), true).unwrap();
        }
        Err(e) => {
            println!("Failed to download python bot: {}", e);
            return Err(format!("Failed to download python bot: {}", e));
        }
    }

    let bundles = scan_directory_for_bot_configs(top_dir.to_str().unwrap());
    let bundle = bundles.iter().next().unwrap();
    let config_file = bundle.path.clone().unwrap();
    let python_file = bundle.python_path.clone();

    let config = Ini::from_file(&config_file).unwrap().section(BOT_CONFIG_MODULE_HEADER).item(PYTHON_FILE_KEY, bot_name);
    config.to_file(&config_file).unwrap();

    if !get_command_status(&python_file, vec![]) {
        println!("You have no default program to open .py files. Your new bot is located at {}", top_dir.to_str().unwrap());
    }

    Ok(config_file)
}

#[tauri::command]
async fn begin_python_bot(bot_name: String) -> HashMap<String, String> {
    let bot_directory = ensure_bot_directory();

    match bootstrap_python_bot(bot_name, &bot_directory) {
        Ok(config_file) => {
            let bundle = BotConfigBundle::from_path(Path::new(&config_file)).unwrap();
            HashMap::from([("bots".to_string(), serde_json::to_string(&bundle).unwrap())])
        }
        Err(e) => HashMap::from([("err".to_string(), e)]),
    }
}

fn main() {
    initialize(&CONFIG_PATH);
    initialize(&BOT_FOLDER_SETTINGS);
    initialize(&MATCH_SETTINGS);
    initialize(&PYTHON_PATH);

    let tray_menu = SystemTrayMenu::new(); // insert the menu items here
    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(system_tray)
        .invoke_handler(tauri::generate_handler![
            get_folder_settings,
            save_folder_settings,
            pick_bot_folder,
            pick_bot_config,
            show_path_in_explorer,
            scan_for_bots,
            get_looks,
            save_looks,
            scan_for_scripts,
            get_match_options,
            get_match_settings,
            save_match_settings,
            get_team_settings,
            save_team_settings,
            get_language_support,
            get_python_path,
            set_python_path,
            get_recommendations,
            pick_appearance_file,
            begin_python_bot,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
