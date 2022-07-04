use crate::{
    custom_maps::find_all_custom_maps,
    rlbot::{
        agents::runnable::Runnable,
        parsing::{
            agent_config_parser::BotLooksConfig,
            bot_config_bundle::{BotConfigBundle, ScriptConfigBundle},
            directory_scanner::{scan_directory_for_bot_configs, scan_directory_for_script_configs},
            match_settings_config_parser::*,
        },
    },
    settings::*,
    *,
};
use configparser::ini::Ini;
use glob::glob;
use rayon::iter::{IntoParallelRefIterator, ParallelExtend, ParallelIterator};
use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir_all, read_to_string},
    path::Path,
    process::Command,
};
use tauri::{api::dialog::FileDialogBuilder, Window};

pub fn load_gui_config(window: &Window) -> Ini {
    let mut conf = Ini::new();
    conf.set_comment_symbols(&[';']);
    let config_path = get_config_path();

    if !config_path.exists() {
        if let Err(e) = create_dir_all(config_path.parent().unwrap()) {
            ccprintlne(window, format!("Failed to create config directory: {}", e));
        }

        conf.set("bot_folder_settings", "files", Some("{}".to_owned()));
        conf.set("bot_folder_settings", "folders", Some("{}".to_owned()));
        conf.set("bot_folder_settings", "incr", None);
        conf.set("match_settings", "map", Some(MAP_TYPES[0].to_owned()));
        conf.set("match_settings", "game_mode", Some(GAME_MODES[0].to_owned()));
        conf.set("match_settings", "match_behavior", Some(EXISTING_MATCH_BEHAVIOR_TYPES[0].to_owned()));
        conf.set("match_settings", "skip_replays", Some("false".to_owned()));
        conf.set("match_settings", "instant_start", Some("false".to_owned()));
        conf.set("match_settings", "enable_lockstep", Some("false".to_owned()));
        conf.set("match_settings", "randomize_map", Some("false".to_owned()));
        conf.set("match_settings", "enable_rendering", Some("false".to_owned()));
        conf.set("match_settings", "enable_state_setting", Some("true".to_owned()));
        conf.set("match_settings", "auto_save_replay", Some("false".to_owned()));
        conf.set("match_settings", "scripts", Some("[]".to_owned()));
        conf.set("mutator_settings", "match_length", Some(MATCH_LENGTH_TYPES[0].to_owned()));
        conf.set("mutator_settings", "max_score", Some(MAX_SCORE_TYPES[0].to_owned()));
        conf.set("mutator_settings", "overtime", Some(OVERTIME_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "series_length", Some(SERIES_LENGTH_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "game_speed", Some(GAME_SPEED_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "ball_max_speed", Some(BALL_MAX_SPEED_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "ball_type", Some(BALL_TYPE_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "ball_weight", Some(BALL_WEIGHT_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "ball_size", Some(BALL_SIZE_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "ball_bounciness", Some(BALL_BOUNCINESS_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "boost_amount", Some(BOOST_AMOUNT_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "rumble", Some(RUMBLE_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "boost_strength", Some(BOOST_STRENGTH_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "gravity", Some(GRAVITY_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "demolish", Some(DEMOLISH_MUTATOR_TYPES[0].to_owned()));
        conf.set("mutator_settings", "respawn_time", Some(RESPAWN_TIME_MUTATOR_TYPES[0].to_owned()));
        conf.set("python_config", "path", Some(auto_detect_python().unwrap_or_default().0));
        conf.set("launcher_settings", "preferred_launcher", Some("epic".to_owned()));
        conf.set("launcher_settings", "use_login_tricks", Some("true".to_owned()));
        conf.set("launcher_settings", "rocket_league_exe_path", None);

        if let Err(e) = conf.write(&config_path) {
            ccprintlne(window, format!("Failed to write config file: {}", e));
        }
    } else if let Err(e) = conf.load(config_path) {
        ccprintlne(window, format!("Failed to load config: {}", e));
    }

    conf
}

#[tauri::command]
pub async fn save_folder_settings(window: Window, bot_folder_settings: BotFolderSettings) {
    BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().update_config(&window, bot_folder_settings)
}

#[tauri::command]
pub async fn get_folder_settings() -> BotFolderSettings {
    BOT_FOLDER_SETTINGS.lock().unwrap().as_ref().unwrap().clone()
}

fn filter_hidden_bundles<T: Runnable + Clone>(bundles: HashSet<T>) -> Vec<T> {
    bundles.iter().filter(|b| !b.get_config_file_name().starts_with('_')).cloned().collect()
}

fn get_bots_from_directory(path: &str) -> Vec<BotConfigBundle> {
    filter_hidden_bundles(scan_directory_for_bot_configs(path))
}

#[tauri::command]
pub async fn scan_for_bots() -> Vec<BotConfigBundle> {
    let bfs_lock = BOT_FOLDER_SETTINGS.lock().unwrap();
    let bfs = bfs_lock.as_ref().unwrap();
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
pub async fn scan_for_scripts() -> Vec<ScriptConfigBundle> {
    let bfs_lock = BOT_FOLDER_SETTINGS.lock().unwrap();
    let bfs = bfs_lock.as_ref().unwrap();
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

#[tauri::command]
pub async fn pick_bot_folder(window: Window) {
    FileDialogBuilder::new().pick_folder(move |folder_path| {
        if let Some(path) = folder_path {
            BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_folder(&window, path.to_string_lossy().to_string());
        }
    });
}

#[tauri::command]
pub async fn pick_bot_config(window: Window) {
    FileDialogBuilder::new().add_filter("Bot Cfg File", &["cfg"]).pick_file(move |path| {
        if let Some(path) = path {
            BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_file(&window, path.to_string_lossy().to_string());
        }
    });
}

#[tauri::command]
pub async fn show_path_in_explorer(window: Window, path: String) {
    let command = if cfg!(target_os = "windows") {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let ppath = Path::new(&*path);
    let path = if ppath.is_file() { ppath.parent().unwrap().to_string_lossy().to_string() } else { path };

    if let Err(e) = Command::new(command).arg(&path).spawn() {
        ccprintlne(&window, format!("Failed to open path: {}", e));
    }
}

#[tauri::command]
pub async fn get_looks(path: String) -> Option<BotLooksConfig> {
    match BotLooksConfig::from_path(&*path) {
        Ok(looks) => Some(looks),
        Err(_) => None,
    }
}

#[tauri::command]
pub async fn save_looks(window: Window, path: String, config: BotLooksConfig) {
    config.save_to_path(&window, &*path);
}

#[tauri::command]
pub async fn get_match_options() -> MatchOptions {
    let mut mo = MatchOptions::default();
    mo.map_types.extend(find_all_custom_maps(&BOT_FOLDER_SETTINGS.lock().unwrap().as_ref().unwrap().folders));
    mo
}

#[tauri::command]
pub async fn get_match_settings(window: Window) -> MatchSettings {
    MatchSettings::load(&window)
}

#[tauri::command]
pub async fn save_match_settings(window: Window, settings: MatchSettings) {
    settings.cleaned_scripts().save_config(&window);
}

#[tauri::command]
pub async fn get_team_settings(window: Window) -> HashMap<String, Vec<BotConfigBundle>> {
    let config = load_gui_config(&window);
    let blue_team = serde_json::from_str(
        &config
            .get("team_settings", "blue_team")
            .unwrap_or_else(|| "[{\"name\": \"Human\", \"runnable_type\": \"human\", \"image\": \"imgs/human.png\"}]".to_owned()),
    )
    .unwrap_or_default();
    let orange_team = serde_json::from_str(&config.get("team_settings", "orange_team").unwrap_or_else(|| "[]".to_owned())).unwrap_or_default();

    let mut bots = HashMap::new();
    bots.insert("blue_team".to_owned(), blue_team);
    bots.insert("orange_team".to_owned(), orange_team);

    bots
}

#[tauri::command]
pub async fn save_team_settings(window: Window, blue_team: Vec<BotConfigBundle>, orange_team: Vec<BotConfigBundle>) {
    let mut config = load_gui_config(&window);
    config.set("team_settings", "blue_team", Some(serde_json::to_string(&clean(blue_team)).unwrap()));
    config.set("team_settings", "orange_team", Some(serde_json::to_string(&clean(orange_team)).unwrap()));

    if let Err(e) = config.write(get_config_path()) {
        ccprintlne(&window, format!("Failed to save team settings: {}", e));
    }
}

#[tauri::command]
pub async fn get_language_support() -> HashMap<String, bool> {
    let mut lang_support = HashMap::new();

    lang_support.insert("java".to_owned(), get_command_status("java", vec!["-version"]));
    lang_support.insert("node".to_owned(), get_command_status("node", vec!["--version"]));
    lang_support.insert("chrome".to_owned(), has_chrome());
    lang_support.insert("fullpython".to_owned(), get_command_status(&PYTHON_PATH.lock().unwrap(), vec!["-c", "import tkinter"]));

    dbg!(lang_support)
}

#[tauri::command]
pub async fn get_detected_python_path() -> Option<(String, bool)> {
    auto_detect_python()
}

#[tauri::command]
pub async fn get_python_path() -> String {
    PYTHON_PATH.lock().unwrap().to_owned()
}

#[tauri::command]
pub async fn set_python_path(window: Window, path: String) {
    *PYTHON_PATH.lock().unwrap() = path.to_owned();
    let mut config = load_gui_config(&window);
    config.set("python_config", "path", Some(path));

    if let Err(e) = config.write(get_config_path()) {
        ccprintlne(&window, format!("Failed to save python path: {}", e));
    }
}

#[tauri::command]
pub async fn pick_appearance_file(window: Window) {
    FileDialogBuilder::new().add_filter("Appearance Cfg File", &["cfg"]).pick_file(move |path| {
        if let Some(path) = path {
            if let Err(e) = window.emit("set_appearance_file", path.to_string_lossy().to_string()) {
                ccprintlne(&window, format!("Failed to set appearance file: {}", e));
            }
        }
    });
}

fn get_recommendations_json(window: &Window) -> Option<AllRecommendations<String>> {
    // Search for and load the json file
    for path in BOT_FOLDER_SETTINGS.lock().unwrap().as_ref().unwrap().folders.keys() {
        let pattern = Path::new(path).join("**/recommendations.json");

        for path2 in glob(&pattern.to_string_lossy().to_owned()).unwrap().flatten() {
            let raw_json = match read_to_string(&path2) {
                Ok(s) => s,
                Err(_) => {
                    ccprintlne(window, format!("Failed to read {}", path2.to_string_lossy()));
                    continue;
                }
            };

            match serde_json::from_str(&raw_json) {
                Ok(j) => return Some(j),
                Err(e) => {
                    ccprintlne(window, format!("Failed to parse file {}: {}", path2.to_string_lossy(), e));
                    continue;
                }
            }
        }
    }

    None
}

#[tauri::command]
pub async fn get_recommendations(window: Window) -> Option<AllRecommendations<BotConfigBundle>> {
    // If we found the json, return the corresponding BotConfigBundles for the bots
    get_recommendations_json(&window).map(|j| {
        // Get a list of all the bots in (bot name, bot config file path) pairs
        let name_path_pairs = {
            let bfs_lock = BOT_FOLDER_SETTINGS.lock().unwrap();
            let bfs = bfs_lock.as_ref().unwrap();
            let mut bots = Vec::new();

            bots.par_extend(
                bfs.folders
                    .par_iter()
                    .filter_map(|(path, props)| {
                        if props.visible {
                            let pattern = Path::new(path).join("**/*.cfg");
                            let paths = glob(&pattern.to_string_lossy().to_owned()).unwrap().flatten().collect::<Vec<_>>();

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
        j.change_generic(&|bot_name| {
            for (name, path) in &name_path_pairs {
                if name == bot_name {
                    if let Ok(mut bundle) = BotConfigBundle::minimal_from_path(Path::new(path)) {
                        bundle.logo = bundle.get_logo();

                        if has_rlbot {
                            let missing_packages = bundle.get_missing_packages(&window);
                            if !missing_packages.is_empty() {
                                bundle.warn = Some("pythonpkg".to_owned());
                            }
                            bundle.missing_python_packages = Some(missing_packages);
                        }

                        return bundle;
                    }
                }
            }

            BotConfigBundle::default()
        })
    })
}
