#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod bot_management;
mod custom_maps;
mod rlbot;

use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fs::{create_dir_all, read_to_string, write, File},
    io::{copy, Cursor, Read},
    ops::Not,
    path::{Path, PathBuf},
    process::{ChildStderr, ChildStdout, Command, Stdio},
    str::FromStr,
    sync::Arc,
    thread,
    time::Duration,
};

use bot_management::{
    bot_creation::{bootstrap_python_bot, bootstrap_python_hivemind, bootstrap_rust_bot, bootstrap_scratch_bot, CREATED_BOTS_FOLDER},
    downloader::{download_repo, BotpackStatus},
};
use glob::glob;

use custom_maps::find_all_custom_maps;
use lazy_static::{initialize, lazy_static};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelExtend, ParallelIterator};
use rlbot::parsing::{
    agent_config_parser::BotLooksConfig,
    bot_config_bundle::{BotConfigBundle, Clean, ScriptConfigBundle},
    directory_scanner::scan_directory_for_script_configs,
};
use rlbot::{agents::runnable::Runnable, parsing::match_settings_config_parser::*};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Manager, Window};

use configparser::ini::Ini;

use rlbot::parsing::directory_scanner::scan_directory_for_bot_configs;

const BOTPACK_FOLDER: &str = "RLBotPackDeletable";
const MAPPACK_FOLDER: &str = "RLBotMapPackDeletable";
const MAPPACK_REPO: (&str, &str) = ("azeemba", "RLBotMapPack");
const OLD_BOTPACK_FOLDER: &str = "RLBotPack";
const BOTPACK_REPO_OWNER: &str = "RLBot";
const BOTPACK_REPO_NAME: &str = "RLBotPack";
const BOTPACK_REPO_BRANCH: &str = "master"; // can't change with the new release system

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
    fn from_path<T: AsRef<Path>>(path: T) -> Self {
        let mut conf = Ini::new();
        conf.load(path).unwrap();
        let files = serde_json::from_str(&conf.get("bot_folder_settings", "files").unwrap_or_else(|| String::from("[]"))).unwrap_or_default();

        let folders = serde_json::from_str(&*conf.get("bot_folder_settings", "folders").unwrap_or_else(|| String::from("[]"))).unwrap_or_default();

        Self { files, folders }
    }

    fn update_config(&mut self, bfs: Self) {
        self.files = bfs.files;
        self.folders = bfs.folders;

        let path = get_config_path();
        let mut conf = Ini::new();
        conf.load(&path).unwrap();
        conf.set("bot_folder_settings", "files", serde_json::to_string(&self.files).ok());
        conf.set("bot_folder_settings", "folders", serde_json::to_string(&self.folders).ok());
        conf.write(&path).unwrap();
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
    fn from_path<T: AsRef<Path>>(path: T) -> Self {
        let mut conf = Ini::new();
        conf.load(path).unwrap();

        let match_length = conf.get("mutator_settings", "match_length").unwrap_or_else(|| MATCH_LENGTH_TYPES[0].to_string());
        let max_score = conf.get("mutator_settings", "max_score").unwrap_or_else(|| MAX_SCORE_TYPES[0].to_string());
        let overtime = conf.get("mutator_settings", "overtime").unwrap_or_else(|| OVERTIME_MUTATOR_TYPES[0].to_string());
        let series_length = conf.get("mutator_settings", "series_length").unwrap_or_else(|| SERIES_LENGTH_MUTATOR_TYPES[0].to_string());
        let game_speed = conf.get("mutator_settings", "game_speed").unwrap_or_else(|| GAME_SPEED_MUTATOR_TYPES[0].to_string());
        let ball_max_speed = conf
            .get("mutator_settings", "ball_max_speed")
            .unwrap_or_else(|| BALL_MAX_SPEED_MUTATOR_TYPES[0].to_string());
        let ball_type = conf.get("mutator_settings", "ball_type").unwrap_or_else(|| BALL_TYPE_MUTATOR_TYPES[0].to_string());
        let ball_weight = conf.get("mutator_settings", "ball_weight").unwrap_or_else(|| BALL_WEIGHT_MUTATOR_TYPES[0].to_string());
        let ball_size = conf.get("mutator_settings", "ball_size").unwrap_or_else(|| BALL_SIZE_MUTATOR_TYPES[0].to_string());
        let ball_bounciness = conf
            .get("mutator_settings", "ball_bounciness")
            .unwrap_or_else(|| BALL_BOUNCINESS_MUTATOR_TYPES[0].to_string());
        let boost_amount = conf.get("mutator_settings", "boost_amount").unwrap_or_else(|| BOOST_AMOUNT_MUTATOR_TYPES[0].to_string());
        let rumble = conf.get("mutator_settings", "rumble").unwrap_or_else(|| RUMBLE_MUTATOR_TYPES[0].to_string());
        let boost_strength = conf
            .get("mutator_settings", "boost_strength")
            .unwrap_or_else(|| BOOST_STRENGTH_MUTATOR_TYPES[0].to_string());
        let gravity = conf.get("mutator_settings", "gravity").unwrap_or_else(|| GRAVITY_MUTATOR_TYPES[0].to_string());
        let demolish = conf.get("mutator_settings", "demolish").unwrap_or_else(|| DEMOLISH_MUTATOR_TYPES[0].to_string());
        let respawn_time = conf.get("mutator_settings", "respawn_time").unwrap_or_else(|| RESPAWN_TIME_MUTATOR_TYPES[0].to_string());

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

        let path = get_config_path();
        let mut conf = Ini::new();
        conf.load(&path).unwrap();
        conf.set("mutator_settings", "match_length", Some(self.match_length.clone()));
        conf.set("mutator_settings", "max_score", Some(self.max_score.clone()));
        conf.set("mutator_settings", "overtime", Some(self.overtime.clone()));
        conf.set("mutator_settings", "series_length", Some(self.series_length.clone()));
        conf.set("mutator_settings", "game_speed", Some(self.game_speed.clone()));
        conf.set("mutator_settings", "ball_max_speed", Some(self.ball_max_speed.clone()));
        conf.set("mutator_settings", "ball_type", Some(self.ball_type.clone()));
        conf.set("mutator_settings", "ball_weight", Some(self.ball_weight.clone()));
        conf.set("mutator_settings", "ball_size", Some(self.ball_size.clone()));
        conf.set("mutator_settings", "ball_bounciness", Some(self.ball_bounciness.clone()));
        conf.set("mutator_settings", "boost_amount", Some(self.boost_amount.clone()));
        conf.set("mutator_settings", "rumble", Some(self.rumble.clone()));
        conf.set("mutator_settings", "boost_strength", Some(self.boost_strength.clone()));
        conf.set("mutator_settings", "gravity", Some(self.gravity.clone()));
        conf.set("mutator_settings", "demolish", Some(self.demolish.clone()));
        conf.set("mutator_settings", "respawn_time", Some(self.respawn_time.clone()));
        conf.write(&path).unwrap();
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
    fn from_path<T: AsRef<Path>>(path: T) -> Self {
        let mut conf = Ini::new();
        conf.load(&path).unwrap();

        let map = conf.get("match_settings", "map").unwrap_or_else(|| MAP_TYPES[0].to_string());
        let game_mode = conf.get("match_settings", "game_mode").unwrap_or_else(|| GAME_MODES[0].to_string());
        let match_behavior = conf.get("match_settings", "match_behavior").unwrap_or_else(|| EXISTING_MATCH_BEHAVIOR_TYPES[0].to_string());
        let skip_replays = conf.getbool("match_settings", "skip_replays").unwrap_or(Some(false)).unwrap_or(false);
        let instant_start = conf.getbool("match_settings", "instant_start").unwrap_or(Some(false)).unwrap_or(false);
        let enable_lockstep = conf.getbool("match_settings", "enable_lockstep").unwrap_or(Some(false)).unwrap_or(false);
        let randomize_map = conf.getbool("match_settings", "randomize_map").unwrap_or(Some(false)).unwrap_or(false);
        let enable_rendering = conf.getbool("match_settings", "enable_rendering").unwrap_or(Some(false)).unwrap_or(false);
        let enable_state_setting = conf.getbool("match_settings", "enable_state_setting").unwrap_or(Some(true)).unwrap_or(true);
        let auto_save_replay = conf.getbool("match_settings", "auto_save_replay").unwrap_or(Some(false)).unwrap_or(false);
        let scripts = serde_json::from_str(&conf.get("match_settings", "scripts").unwrap_or_else(|| "[]".to_string())).unwrap_or_default();

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
            scripts,
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

        let path = get_config_path();
        let mut conf = Ini::new();
        conf.load(&path).unwrap();
        conf.set("match_settings", "map", Some(self.map.clone()));
        conf.set("match_settings", "game_mode", Some(self.game_mode.clone()));
        conf.set("match_settings", "match_behavior", Some(self.match_behavior.clone()));
        conf.set("match_settings", "skip_replays", Some(self.skip_replays.to_string()));
        conf.set("match_settings", "instant_start", Some(self.instant_start.to_string()));
        conf.set("match_settings", "enable_lockstep", Some(self.enable_lockstep.to_string()));
        conf.set("match_settings", "randomize_map", Some(self.randomize_map.to_string()));
        conf.set("match_settings", "enable_rendering", Some(self.enable_rendering.to_string()));
        conf.set("match_settings", "enable_state_setting", Some(self.enable_state_setting.to_string()));
        conf.set("match_settings", "auto_save_replay", Some(self.auto_save_replay.to_string()));
        conf.set("match_settings", "scripts", Some(serde_json::to_string(&self.scripts).unwrap_or_default()));
        conf.write(&path).unwrap();
    }

    fn cleaned_scripts(&self) -> Self {
        let mut new = self.clone();
        new.scripts = clean(new.scripts);
        new
    }
}

#[cfg(windows)]
fn auto_detect_python() -> String {
    let content_folder = get_content_folder();

    match content_folder.join("Python37\\python.exe") {
        path if path.exists() => path.to_str().unwrap().to_string(),
        _ => match content_folder.join("venv\\python.exe") {
            path if path.exists() => path.to_str().unwrap().to_string(),
            _ => "python".to_string(),
        },
    }
}

#[cfg(target_os = "macos")]
fn auto_detect_python() -> String {
    "python3.7".to_string()
}

#[cfg(target_os = "linux")]
fn auto_detect_python() -> String {
    match get_content_folder().join("env/bin/python") {
        path if path.exists() => path.to_str().unwrap().to_string(),
        _ => "python3.7".to_string(),
    }
}

fn get_config_path() -> PathBuf {
    get_content_folder().join("config.ini")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ConsoleText {
    pub text: String,
    pub err: bool,
}

impl ConsoleText {
    pub const fn from(text: String, err: bool) -> ConsoleText {
        ConsoleText { text, err }
    }
}

lazy_static! {
    static ref BOT_FOLDER_SETTINGS: Mutex<BotFolderSettings> = Mutex::new(BotFolderSettings::from_path(get_config_path()));
    static ref MATCH_SETTINGS: Mutex<MatchSettings> = Mutex::new(MatchSettings::from_path(get_config_path()));
    static ref PYTHON_PATH: Mutex<String> = Mutex::new({
        let mut config = Ini::new();
        config.load(get_config_path()).unwrap();
        match config.get("python_config", "path") {
            Some(path) => path,
            None => auto_detect_python(),
        }
    });
    static ref CONSOLE_TEXT: Mutex<Vec<ConsoleText>> = Mutex::new(vec![
        ConsoleText::from("Welcome to the RLBot Console!".to_string(), false),
        ConsoleText::from("".to_string(), false)
    ]);
    static ref STDOUT_CAPTURE: Arc<Mutex<Vec<Option<ChildStdout>>>> = Arc::new(Mutex::new(Vec::new()));
    static ref STDERR_CAPTURE: Arc<Mutex<Vec<Option<ChildStderr>>>> = Arc::new(Mutex::new(Vec::new()));
}

pub fn ccprintln(text: String) {
    CONSOLE_TEXT.lock().unwrap().push(ConsoleText::from(text, false));
}

fn check_has_rlbot() -> bool {
    get_command_status(&*PYTHON_PATH.lock().unwrap(), vec!["-c", "import rlbot"])
}

#[cfg(windows)]
fn get_content_folder() -> PathBuf {
    PathBuf::from(format!("{}\\RLBotGUIX", env::var("LOCALAPPDATA").unwrap()))
}

#[cfg(target_os = "macos")]
fn get_content_folder() -> PathBuf {
    PathBuf::from(format!("{}/Library/Application Support/rlbotgui", env::var("HOME").unwrap()))
}

#[cfg(target_os = "linux")]
fn get_content_folder() -> PathBuf {
    PathBuf::from(format!("{}/.RLBotGUI", env::var("HOME").unwrap()))
}

fn get_missing_packages_script_path() -> PathBuf {
    get_content_folder().join("get_missing_packages.py")
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

#[tauri::command]
async fn scan_for_bots() -> Vec<BotConfigBundle> {
    let bfs = BOT_FOLDER_SETTINGS.lock().unwrap();
    let mut bots = Vec::new();

    for (path, props) in bfs.folders.iter() {
        if props.visible {
            bots.extend(get_bots_from_directory(&*path));
        }
    }

    for (path, props) in bfs.files.iter() {
        if props.visible {
            if let Ok(bundle) = BotConfigBundle::minimal_from_path(Path::new(path)) {
                bots.push(bundle);
            }
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
            if let Ok(bundle) = ScriptConfigBundle::minimal_from_path(Path::new(path)) {
                scripts.push(bundle);
            }
        }
    }

    scripts
}

use native_dialog::FileDialog;

#[cfg(not(target_os = "macos"))]
#[tauri::command]
async fn pick_bot_folder() {
    let path = match FileDialog::new().show_open_single_dir().unwrap() {
        Some(path) => path,
        None => return,
    };

    BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(path.to_str().unwrap().to_string());
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn pick_bot_folder(window: Window) {
    // FileDialog must be ran on the main thread when running on MacOS, it will panic if it isn't
    window
        .run_on_main_thread(|| {
            let path = match FileDialog::new().show_open_single_dir().unwrap() {
                Some(path) => path,
                None => return,
            };

            BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(path.to_str().unwrap().to_string());
        })
        .unwrap();
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

    Command::new(command).arg(path).spawn().unwrap();
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
    MATCH_SETTINGS.lock().unwrap().clone()
}

#[tauri::command]
async fn save_match_settings(settings: MatchSettings) {
    MATCH_SETTINGS.lock().unwrap().update_config(settings.cleaned_scripts());
}

#[tauri::command]
async fn get_team_settings() -> HashMap<String, Vec<BotConfigBundle>> {
    let mut config = Ini::new();
    config.load(get_config_path()).unwrap();
    let blue_team = serde_json::from_str(
        &config
            .get("team_settings", "blue_team")
            .unwrap_or_else(|| "[{\"name\": \"Human\", \"type_\": \"human\", \"image\": \"imgs/human.png\"}]".to_string()),
    )
    .unwrap_or_default();
    let orange_team = serde_json::from_str(&config.get("team_settings", "orange_team").unwrap_or_else(|| "[]".to_string())).unwrap_or_default();

    let mut bots = HashMap::new();
    bots.insert("blue_team".to_string(), blue_team);
    bots.insert("orange_team".to_string(), orange_team);

    bots
}

fn clean<T: Clean>(items: Vec<T>) -> Vec<T> {
    items.iter().map(|i| i.cleaned()).collect()
}

#[tauri::command]
async fn save_team_settings(blue_team: Vec<BotConfigBundle>, orange_team: Vec<BotConfigBundle>) {
    let config_path = get_config_path();
    let mut config = Ini::new();
    config.load(&config_path).unwrap();
    config.set("team_settings", "blue_team", Some(serde_json::to_string(&clean(blue_team)).unwrap()));
    config.set("team_settings", "orange_team", Some(serde_json::to_string(&clean(orange_team)).unwrap()));
    config.write(&config_path).unwrap();
}

fn get_command_status(program: &str, args: Vec<&str>) -> bool {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // disable window creation
        command.creation_flags(0x08000000);
    };

    match command.args(args).stdout(Stdio::null()).stderr(Stdio::null()).status() {
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
    get_command_status("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", vec!["--version"])
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

    if get_command_status(&python_path, vec!["--version"]) {
        lang_support.insert("python".to_string(), true);
        lang_support.insert("fullpython".to_string(), get_command_status(&*python_path, vec!["-c", "import tkinter"]));
        lang_support.insert(
            "rlbotpython".to_string(),
            get_command_status(&python_path, vec!["-c", "import rlbot; import numpy; import numba; import scipy; import selenium"]),
        );
    } else {
        lang_support.insert("python".to_string(), false);
        lang_support.insert("fullpython".to_string(), false);
        lang_support.insert("rlbotpython".to_string(), false);
    }

    dbg!(lang_support)
}

#[tauri::command]
async fn get_detected_python_path() -> Option<String> {
    let python = auto_detect_python();
    if get_command_status(&python, vec!["--version"]) {
        Some(python)
    } else {
        None
    }
}

#[tauri::command]
async fn get_python_path() -> String {
    PYTHON_PATH.lock().unwrap().to_string()
}

#[tauri::command]
async fn set_python_path(path: String) {
    *PYTHON_PATH.lock().unwrap() = path.clone();
    let config_path = get_config_path();
    let mut config = Ini::new();
    config.load(&config_path).unwrap();
    config.set("python_config", "path", Some(path));
    config.write(&config_path).unwrap();
}

#[cfg(not(target_os = "macos"))]
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

#[cfg(target_os = "macos")]
#[tauri::command]
async fn pick_appearance_file(window: Window) -> Option<String> {
    // FileDialog must be ran on the main thread when running on MacOS, it will panic if it isn't
    let out = Arc::new(Mutex::new(None));
    let out_clone = Arc::clone(&out);
    window
        .run_on_main_thread(move || {
            let mut out_ref = out_clone.lock().unwrap();
            *out_ref = match FileDialog::new().add_filter("Appearance Cfg File", &["cfg"]).show_open_single_file() {
                Ok(path) => path.map(|path| path.to_str().unwrap().to_string()),
                Err(e) => {
                    dbg!(e);
                    None
                }
            };
        })
        .unwrap();

    // Rust requries that we first store the clone in a variable before we return it so out can be dropped safely
    let x = out.lock().unwrap().clone();
    x
}

type BotNames = Vec<String>;
type Recommendation = HashMap<String, BotNames>;
type AllRecommendations = HashMap<String, Vec<Recommendation>>;

fn get_recommendations_json() -> Option<AllRecommendations> {
    // Search for and load the json file
    for path in BOT_FOLDER_SETTINGS.lock().unwrap().folders.keys() {
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
                Ok(j) => return Some(j),
                Err(e) => {
                    println!("Failed to parse file {}: {}", path2.to_str().unwrap(), e);
                    continue;
                }
            }
        }
    }

    None
}

#[tauri::command]
async fn get_recommendations() -> Option<HashMap<String, Vec<HashMap<String, Vec<BotConfigBundle>>>>> {
    // If we found the json, return the corresponding BotConfigBundles for the bots
    get_recommendations_json().map(|j| {
        // Get a list of all the bots in (bot name, bot config file path) pairs
        let name_path_pairs = {
            let bfs = BOT_FOLDER_SETTINGS.lock().unwrap();
            let mut bots = Vec::new();

            bots.par_extend(
                bfs.folders
                    .par_iter()
                    .filter_map(|(path, props)| {
                        if props.visible {
                            let pattern = Path::new(path).join("**/*.cfg");
                            let paths = glob(pattern.to_str().unwrap()).unwrap().flatten().collect::<Vec<_>>();

                            Some(paths.par_iter().filter_map(|path| BotConfigBundle::name_from_path(path.as_path()).ok()).collect::<Vec<_>>())
                        } else {
                            None
                        }
                    })
                    .flatten(),
            );

            bots.par_extend(
                bfs.files
                    .par_iter()
                    .filter_map(|(path, props)| if props.visible { BotConfigBundle::name_from_path(Path::new(path)).ok() } else { None }),
            );

            bots
        };

        let has_rlbot = check_has_rlbot();

        // Load all of the bot config bundles
        let recommendations: Vec<HashMap<String, Vec<BotConfigBundle>>> = j
            .get("recommendations")
            .unwrap()
            .par_iter()
            .map(|bots| {
                HashMap::from([(
                    "bots".to_string(),
                    bots.get("bots")
                        .unwrap()
                        .par_iter()
                        .filter_map(|bot_name| {
                            for (name, path) in &name_path_pairs {
                                if name == bot_name {
                                    let mut b = BotConfigBundle::minimal_from_path(Path::new(path)).ok();
                                    if let Some(ib) = b.as_mut() {
                                        ib.logo = ib.get_logo();

                                        if has_rlbot {
                                            let missing_packages = ib.get_missing_packages();
                                            if !missing_packages.is_empty() {
                                                ib.warn = Some("pythonpkg".to_string());
                                            }
                                            ib.missing_python_packages = Some(missing_packages);
                                        }
                                    }
                                    return b;
                                }
                            }

                            None
                        })
                        .collect(),
                )])
            })
            .collect();

        HashMap::from([("recommendations".to_string(), recommendations)])
    })
}

fn ensure_bot_directory() -> String {
    let bot_directory = get_content_folder();
    let bot_directory_path = Path::new(&bot_directory).join(CREATED_BOTS_FOLDER);

    if !bot_directory_path.exists() {
        create_dir_all(&bot_directory_path).unwrap();
    }

    bot_directory.to_str().unwrap().to_string()
}

#[tauri::command]
async fn begin_python_bot(bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_python_bot(bot_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[tauri::command]
async fn begin_python_hivemind(hive_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_python_hivemind(hive_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[tauri::command]
async fn begin_rust_bot(bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_rust_bot(bot_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[tauri::command]
async fn begin_scratch_bot(bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_scratch_bot(bot_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageResult {
    exit_code: i32,
    packages: Vec<String>,
}

fn spawn_capture_process_and_get_exit_code<S: AsRef<OsStr>>(program: S, args: &[&str]) -> i32 {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // disable window creation
        command.creation_flags(0x08000000);
    };

    let mut child = if let Ok(the_child) = command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        the_child
    } else {
        return 2;
    };

    let stdout_index = {
        let mut stdout_capture = STDOUT_CAPTURE.lock().unwrap();
        if let Some(index) = stdout_capture.iter().position(|c| c.is_none()) {
            stdout_capture[index] = Some(child.stdout.take().unwrap());
            index
        } else {
            stdout_capture.push(Some(child.stdout.take().unwrap()));
            stdout_capture.len() - 1
        }
    };

    let stderr_index = {
        let mut stderr_capture = STDERR_CAPTURE.lock().unwrap();
        if let Some(index) = stderr_capture.iter().position(|c| c.is_none()) {
            stderr_capture[index] = Some(child.stderr.take().unwrap());
            index
        } else {
            stderr_capture.push(Some(child.stderr.take().unwrap()));
            stderr_capture.len() - 1
        }
    };

    let exit_code = child.wait().unwrap().code().unwrap_or(1);
    STDOUT_CAPTURE.lock().unwrap()[stdout_index] = None;
    STDERR_CAPTURE.lock().unwrap()[stderr_index] = None;
    exit_code
}

#[tauri::command]
async fn install_package(package_string: String) -> PackageResult {
    let exit_code = spawn_capture_process_and_get_exit_code(
        PYTHON_PATH.lock().unwrap().to_string(),
        &["-m", "pip", "install", "-U", "--no-warn-script-location", &package_string],
    );

    PackageResult {
        exit_code,
        packages: vec![package_string],
    }
}

#[tauri::command]
async fn install_requirements(config_path: String) -> PackageResult {
    let bundle = BotConfigBundle::minimal_from_path(Path::new(&config_path)).unwrap();

    if let Some(file) = bundle.get_requirements_file() {
        let packages = bundle.get_missing_packages();
        let python = PYTHON_PATH.lock().unwrap().to_string();
        let exit_code = spawn_capture_process_and_get_exit_code(&python, &["-m", "pip", "install", "-U", "--no-warn-script-location", "-r", file]);

        PackageResult { exit_code, packages }
    } else {
        PackageResult {
            exit_code: 1,
            packages: vec!["Unknown file".to_owned()],
        }
    }
}

const INSTALL_BASIC_PACKAGES_ARGS: [&[&str]; 4] = [
    &["-m", "ensurepip"],
    &["-m", "pip", "install", "-U", "--no-warn-script-location", "pip"],
    &["-m", "pip", "install", "-U", "--no-warn-script-location", "setuptools", "wheel"],
    &["-m", "pip", "install", "-U", "--no-warn-script-location", "numpy", "scipy", "numba", "selenium", "rlbot"],
];

fn install_upgrade_basic_packages() -> PackageResult {
    let packages = vec![
        String::from("pip"),
        String::from("setuptools"),
        String::from("wheel"),
        String::from("numpy"),
        String::from("scipy"),
        String::from("numba"),
        String::from("selenium"),
        String::from("rlbot"),
    ];

    let python = PYTHON_PATH.lock().unwrap().to_string();

    let mut exit_code = 0;

    for command in INSTALL_BASIC_PACKAGES_ARGS {
        if exit_code != 0 {
            break;
        }

        exit_code = spawn_capture_process_and_get_exit_code(&python, command);
    }

    PackageResult { exit_code, packages }
}

#[tauri::command]
async fn install_basic_packages() -> PackageResult {
    install_upgrade_basic_packages()
}

#[tauri::command]
async fn get_console_texts() -> Vec<ConsoleText> {
    CONSOLE_TEXT.lock().unwrap().clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MissingPackagesUpdate {
    pub index: usize,
    pub warn: Option<String>,
    pub missing_packages: Option<Vec<String>>,
}

#[tauri::command]
async fn get_missing_bot_packages(bots: Vec<BotConfigBundle>) -> Vec<MissingPackagesUpdate> {
    if check_has_rlbot() {
        bots.par_iter()
            .enumerate()
            .filter_map(|(index, bot)| {
                if bot.type_ == *"rlbot" {
                    let mut warn = bot.warn.clone();
                    let mut missing_packages = bot.missing_python_packages.clone();

                    if let Some(missing_packages) = &missing_packages {
                        if warn == Some("pythonpkg".to_string()) && missing_packages.is_empty() {
                            warn = None;
                        }
                    } else {
                        let bot_missing_packages = bot.get_missing_packages();

                        if !bot_missing_packages.is_empty() {
                            warn = Some("pythonpkg".to_string());
                        } else {
                            warn = None;
                        }

                        missing_packages = Some(bot_missing_packages);
                    }

                    if warn != bot.warn || missing_packages != bot.missing_python_packages {
                        return Some(MissingPackagesUpdate { index, warn, missing_packages });
                    }
                }

                None
            })
            .collect()
    } else {
        bots.par_iter()
            .enumerate()
            .filter_map(|(index, bot)| {
                if bot.type_ == *"rlbot" && (bot.warn.is_some() || bot.missing_python_packages.is_some()) {
                    Some(MissingPackagesUpdate {
                        index,
                        warn: None,
                        missing_packages: None,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[tauri::command]
async fn get_missing_script_packages(scripts: Vec<ScriptConfigBundle>) -> Vec<MissingPackagesUpdate> {
    if check_has_rlbot() {
        scripts
            .par_iter()
            .enumerate()
            .filter_map(|(index, script)| {
                let mut warn = script.warn.clone();
                let mut missing_packages = script.missing_python_packages.clone();

                if let Some(missing_packages) = &missing_packages {
                    if warn == Some("pythonpkg".to_string()) && missing_packages.is_empty() {
                        warn = None;
                    }
                } else {
                    let script_missing_packages = script.get_missing_packages();

                    if !script_missing_packages.is_empty() {
                        warn = Some("pythonpkg".to_string());
                    } else {
                        warn = None;
                    }

                    missing_packages = Some(script_missing_packages);
                }

                if warn != script.warn || missing_packages != script.missing_python_packages {
                    Some(MissingPackagesUpdate { index, warn, missing_packages })
                } else {
                    None
                }
            })
            .collect()
    } else {
        scripts
            .par_iter()
            .enumerate()
            .filter_map(|(index, script)| {
                if script.warn.is_some() || script.missing_python_packages.is_some() {
                    Some(MissingPackagesUpdate {
                        index,
                        warn: None,
                        missing_packages: None,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogoUpdate {
    pub index: usize,
    pub logo: String,
}

#[tauri::command]
async fn get_missing_bot_logos(bots: Vec<BotConfigBundle>) -> Vec<LogoUpdate> {
    bots.par_iter()
        .enumerate()
        .filter_map(|(index, bot)| {
            if bot.type_ == *"rlbot" && bot.logo.is_none() {
                if let Some(logo) = bot.get_logo() {
                    return Some(LogoUpdate { index, logo });
                }
            }

            None
        })
        .collect()
}

#[tauri::command]
async fn get_missing_script_logos(scripts: Vec<ScriptConfigBundle>) -> Vec<LogoUpdate> {
    scripts
        .par_iter()
        .enumerate()
        .filter_map(|(index, script)| {
            if script.logo.is_none() {
                if let Some(logo) = script.get_logo() {
                    return Some(LogoUpdate { index, logo });
                }
            }

            None
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConsoleTextUpdate {
    pub content: ConsoleText,
    pub replace_last: bool,
}

impl ConsoleTextUpdate {
    fn from(text: String, err: bool, replace_last: bool) -> Self {
        ConsoleTextUpdate {
            content: ConsoleText::from(text, err),
            replace_last,
        }
    }
}

#[tauri::command]
fn is_windows() -> bool {
    cfg!(windows)
}

#[tauri::command]
async fn install_python() -> Option<u8> {
    // https://www.python.org/ftp/python/3.7.9/python-3.7.9-amd64.exe
    // download the above file to python-3.7.9-amd64.exe

    let file_path = get_content_folder().join("python-3.7.9-amd64.exe");

    if !file_path.exists() {
        let response = reqwest::get("https://www.python.org/ftp/python/3.7.9/python-3.7.9-amd64.exe").await.ok()?;
        let mut file = File::create(&file_path).ok()?;
        let mut content = Cursor::new(response.bytes().await.ok()?);
        copy(&mut content, &mut file).ok()?;
    }

    // only installs for the current user (requires no admin privileges)
    // adds the Python version to PATH
    // Launches the installer in a simplified mode for a one-button install
    let mut process = Command::new(file_path)
        .args([
            "InstallLauncherAllUsers=0",
            "SimpleInstall=1",
            "PrependPath=1",
            "SimpleInstallDescription='Install Python 3.7.9 for the current user to use with RLBot'",
        ])
        .spawn()
        .ok()?;
    process.wait().ok()?;

    // Windows actually doesn't have a python3.7.exe command, just python.exe (no matter what)
    // but there is a pip3.7.exe
    // Since we added Python to PATH, we can use where to find the path to pip3.7.exe
    // we can then use that to find the path to the right python.exe and use that
    let new_python_path = {
        let output = Command::new("where").arg("pip3.7").output().ok()?;
        let stdout = String::from_utf8(output.stdout).ok()?;
        Path::new(stdout.lines().next()?).parent().unwrap().parent().unwrap().join("python.exe")
    };
    *PYTHON_PATH.lock().unwrap() = new_python_path.to_str().unwrap().to_string();

    Some(0)
}

#[tauri::command]
async fn download_bot_pack(window: Window) {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_str().unwrap().to_string();
    let botpack_status = download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await;
    
    if dbg!(botpack_status) == BotpackStatus::Success {
        // Configure the folder settings
        BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(botpack_location);
    }
}

fn main() {
    initialize(&BOT_FOLDER_SETTINGS);
    initialize(&MATCH_SETTINGS);
    initialize(&PYTHON_PATH);
    initialize(&CONSOLE_TEXT);
    initialize(&STDOUT_CAPTURE);
    initialize(&STDERR_CAPTURE);

    let config_path = get_config_path();
    println!("Config path: {}", config_path.to_str().unwrap());

    if !config_path.exists() {
        create_dir_all(config_path.parent().unwrap()).unwrap();
        let mut conf = Ini::new();
        conf.set("bot_folder_settings", "files", Some("{}".to_string()));
        conf.set("bot_folder_settings", "folders", Some("{}".to_string()));
        conf.set("bot_folder_settings", "incr", None);
        conf.set("match_settings", "map", Some(MAP_TYPES[0].to_string()));
        conf.set("match_settings", "game_mode", Some(GAME_MODES[0].to_string()));
        conf.set("match_settings", "match_behavior", Some(EXISTING_MATCH_BEHAVIOR_TYPES[0].to_string()));
        conf.set("match_settings", "skip_replays", Some("false".to_string()));
        conf.set("match_settings", "instant_start", Some("false".to_string()));
        conf.set("match_settings", "enable_lockstep", Some("false".to_string()));
        conf.set("match_settings", "randomize_map", Some("false".to_string()));
        conf.set("match_settings", "enable_rendering", Some("false".to_string()));
        conf.set("match_settings", "enable_state_setting", Some("true".to_string()));
        conf.set("match_settings", "auto_save_replay", Some("false".to_string()));
        conf.set("match_settings", "scripts", Some("[]".to_string()));
        conf.set("mutator_settings", "match_length", Some(MATCH_LENGTH_TYPES[0].to_string()));
        conf.set("mutator_settings", "max_score", Some(MAX_SCORE_TYPES[0].to_string()));
        conf.set("mutator_settings", "overtime", Some(OVERTIME_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "series_length", Some(SERIES_LENGTH_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "game_speed", Some(GAME_SPEED_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "ball_max_speed", Some(BALL_MAX_SPEED_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "ball_type", Some(BALL_TYPE_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "ball_weight", Some(BALL_WEIGHT_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "ball_size", Some(BALL_SIZE_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "ball_bounciness", Some(BALL_BOUNCINESS_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "boost_amount", Some(BOOST_AMOUNT_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "rumble", Some(RUMBLE_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "boost_strength", Some(BOOST_STRENGTH_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "gravity", Some(GRAVITY_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "demolish", Some(DEMOLISH_MUTATOR_TYPES[0].to_string()));
        conf.set("mutator_settings", "respawn_time", Some(RESPAWN_TIME_MUTATOR_TYPES[0].to_string()));
        conf.set("python_config", "path", Some(auto_detect_python()));

        conf.write(&config_path).unwrap();
    }

    let missing_packages_script_path = get_missing_packages_script_path();
    println!("get_missing_packages.py: {}", missing_packages_script_path.to_str().unwrap());

    if !missing_packages_script_path.parent().unwrap().exists() {
        create_dir_all(&missing_packages_script_path).unwrap();
    }

    write(missing_packages_script_path, include_str!("get_missing_packages.py")).unwrap();

    tauri::Builder::default()
        .setup(|app| {
            let main_window_out = app.get_window("main").unwrap();
            let stdout_capture = Arc::clone(&STDOUT_CAPTURE);
            thread::spawn(move || {
                let mut next_replace_last = false;
                loop {
                    thread::sleep(Duration::from_micros(10));
                    let mut outs = stdout_capture.lock().unwrap();

                    while !outs.is_empty() && outs.last().unwrap().is_none() {
                        outs.pop();
                    }

                    if !outs.is_empty() {
                        let out_strs: Vec<ConsoleTextUpdate> = outs
                            .iter_mut()
                            .flatten()
                            .filter_map(|s| {
                                let mut text = String::new();
                                let mut will_replace_last = next_replace_last;
                                next_replace_last = false;

                                loop {
                                    let mut buf = [0];
                                    match s.read(&mut buf[..]) {
                                        Ok(0) | Err(_) => break,
                                        Ok(_) => {
                                            let string = String::from_utf8_lossy(&buf).to_string();
                                            if &string == "\n" {
                                                if text.is_empty() && will_replace_last {
                                                    will_replace_last = false;
                                                    continue;
                                                }

                                                break;
                                            } else if &string == "\r" {
                                                next_replace_last = true;
                                                break;
                                            }
                                            text.push_str(&string);
                                        }
                                    };
                                }

                                text.is_empty().not().then(|| ConsoleTextUpdate::from(text, false, will_replace_last))
                            })
                            .collect();
                        drop(outs);

                        if !out_strs.is_empty() {
                            let mut console_text = CONSOLE_TEXT.lock().unwrap();
                            for out_str in &out_strs {
                                if out_str.replace_last {
                                    console_text.pop();
                                }
                                console_text.push(out_str.content.clone());
                            }

                            if console_text.len() > 1200 {
                                let diff = console_text.len() - 1200;
                                console_text.drain(..diff);
                            }

                            main_window_out.emit("new-console-text", out_strs).unwrap();
                        }
                    }
                }
            });

            let main_window_err = app.get_window("main").unwrap();
            let stderr_capture = Arc::clone(&STDERR_CAPTURE);
            thread::spawn(move || {
                let mut next_replace_last = false;
                loop {
                    thread::sleep(Duration::from_micros(10));
                    let mut errs = stderr_capture.lock().unwrap();

                    while !errs.is_empty() && errs.last().unwrap().is_none() {
                        errs.pop();
                    }

                    if !errs.is_empty() {
                        let err_strs: Vec<ConsoleTextUpdate> = errs
                            .iter_mut()
                            .flatten()
                            .filter_map(|s| {
                                let mut text = String::new();
                                let mut will_replace_last = next_replace_last;
                                next_replace_last = false;

                                loop {
                                    let mut buf = [0];
                                    match s.read(&mut buf[..]) {
                                        Ok(0) | Err(_) => break,
                                        Ok(_) => {
                                            let string = String::from_utf8_lossy(&buf).to_string();
                                            if &string == "\n" {
                                                if text.is_empty() && will_replace_last {
                                                    will_replace_last = false;
                                                    continue;
                                                }

                                                break;
                                            } else if &string == "\r" {
                                                next_replace_last = true;
                                                break;
                                            }
                                            text.push_str(&string);
                                        }
                                    };
                                }

                                text.is_empty().not().then(|| ConsoleTextUpdate::from(text, true, will_replace_last))
                            })
                            .collect();
                        drop(errs);

                        if !err_strs.is_empty() {
                            let mut console_text = CONSOLE_TEXT.lock().unwrap();
                            for err_str in &err_strs {
                                console_text.push(err_str.content.clone());
                            }
                            if console_text.len() > 1200 {
                                let diff = console_text.len() - 1200;
                                console_text.drain(..diff);
                            }
                            main_window_err.emit("new-console-text", err_strs).unwrap();
                        }
                    }
                }
            });

            Ok(())
        })
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
            begin_python_hivemind,
            begin_rust_bot,
            begin_scratch_bot,
            install_package,
            install_requirements,
            install_basic_packages,
            get_console_texts,
            get_detected_python_path,
            get_missing_bot_packages,
            get_missing_script_packages,
            get_missing_bot_logos,
            get_missing_script_logos,
            is_windows,
            install_python,
            download_bot_pack,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
