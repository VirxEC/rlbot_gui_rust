use crate::{
    bot_management::downloader::MapPackUpdater,
    custom_maps,
    rlbot::{
        agents::runnable::Runnable,
        parsing::{
            agent_config_parser::BotLooksConfig,
            bot_config_bundle::{BotConfigBundle, ScriptConfigBundle},
            directory_scanner::{scan_directory_for_bot_configs, scan_directory_for_script_configs},
            match_settings_config_parser::MatchOptions,
        },
    },
    settings::*,
    stories::{
        bots_base,
        cmaps::{Bot, City, Script, StoryModeConfig},
    },
    *,
};
use configparser::ini::Ini;
use glob::glob;
use std::{
    collections::HashMap,
    fs::{create_dir_all, read_to_string},
    path::Path,
    process::Command,
};
use tauri::{api::dialog::FileDialogBuilder, Window};
use tokio::fs::read_to_string as read_to_string_async;

fn set_gui_config_to_default(conf: &mut Ini) {
    conf.set("bot_folder_settings", "files", Some("{}".to_owned()));
    conf.set("bot_folder_settings", "folders", Some("{}".to_owned()));
    conf.set("bot_folder_settings", "incr", None);
    MatchConfig::default().save_to_config(conf);
    conf.set("python_config", "path", Some(auto_detect_python().unwrap_or_default().0));
    conf.set("launcher_settings", "preferred_launcher", Some("epic".to_owned()));
    conf.set("launcher_settings", "use_login_tricks", Some("true".to_owned()));
    conf.set("launcher_settings", "rocket_league_exe_path", None);
    conf.set("story_mode", "save_state", None);
}

/// Loads the GUI config, creating it if it doesn't exist.
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
pub async fn load_gui_config(window: &Window) -> Ini {
    let mut conf = Ini::new();
    conf.set_comment_symbols(&[';']);
    let config_path = get_config_path();

    if !config_path.exists() {
        if let Err(e) = create_dir_all(config_path.parent().unwrap()) {
            ccprintln!(window, "Error creating config directory: {e}");
        }

        set_gui_config_to_default(&mut conf);

        if let Err(e) = conf.write_async(&config_path).await {
            ccprintln!(window, "Error writing config file: {e}");
        }
    } else if let Err(e) = conf.load_async(config_path).await {
        ccprintln!(window, "Error loading config: {e}");
    }

    conf
}

/// Synchronously loads the GUI config, creating it if it doesn't exist.
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
pub fn load_gui_config_sync(window: &Window) -> Ini {
    let mut conf = Ini::new();
    conf.set_comment_symbols(&[';']);
    let config_path = get_config_path();

    if !config_path.exists() {
        if let Err(e) = create_dir_all(config_path.parent().unwrap()) {
            ccprintln!(window, "Error creating config directory: {e}");
        }

        set_gui_config_to_default(&mut conf);

        if let Err(e) = conf.write(&config_path) {
            ccprintln!(window, "Error writing config file: {e}");
        }
    } else if let Err(e) = conf.load(config_path) {
        ccprintln!(window, "Error loading config: {e}");
    }

    conf
}

#[tauri::command]
pub async fn save_folder_settings(window: Window, bot_folder_settings: BotFolders) -> Result<(), String> {
    BOT_FOLDER_SETTINGS
        .write()
        .map_err(|_| "Mutex BOT_FOLDER_SETTINGS was poisoned")?
        .as_mut()
        .ok_or("BOT_FOLDER_SETTINGS is None")?
        .update_config(&window, bot_folder_settings);
    Ok(())
}

#[tauri::command]
pub async fn get_folder_settings() -> Result<BotFolders, String> {
    Ok(BOT_FOLDER_SETTINGS.read().map_err(|err| err.to_string())?.clone().ok_or("BOT_FOLDER_SETTINGS is None")?)
}

fn filter_hidden_bundles<I>(bundles: I) -> Vec<I::Item>
where
    I: IntoIterator,
    I::Item: Runnable + Clone,
{
    bundles.into_iter().filter(|b| !b.get_config_file_name().starts_with('_')).collect()
}

async fn get_bots_from_directory(window: &Window, path: &str) -> Vec<BotConfigBundle> {
    filter_hidden_bundles(scan_directory_for_bot_configs(window, path).await)
}

#[tauri::command]
pub async fn scan_for_bots(window: Window) -> Result<Vec<BotConfigBundle>, String> {
    let bfs = BOT_FOLDER_SETTINGS
        .read()
        .map_err(|_| "Mutex BOT_FOLDER_SETTINGS was poisoned")?
        .clone()
        .ok_or("BOT_FOLDER_SETTINGS is None")?;
    let mut bots = Vec::new();

    for (path, props) in &bfs.folders {
        if props.visible {
            bots.extend(get_bots_from_directory(&window, &**path).await);
        }
    }

    for (path, props) in &bfs.files {
        if props.visible {
            if let Ok(bundle) = BotConfigBundle::minimal_from_path(Path::new(path)).await {
                bots.push(bundle);
            }
        }
    }

    Ok(bots)
}

async fn get_scripts_from_directory(window: &Window, path: &str) -> Vec<ScriptConfigBundle> {
    filter_hidden_bundles(scan_directory_for_script_configs(window, path).await)
}

#[tauri::command]
pub async fn scan_for_scripts(window: Window) -> Result<Vec<ScriptConfigBundle>, String> {
    let bfs = BOT_FOLDER_SETTINGS
        .read()
        .map_err(|_| "Mutex BOT_FOLDER_SETTINGS was poisoned")?
        .as_ref()
        .ok_or("BOT_FOLDER_SETTINGS is None")?
        .clone();
    let mut scripts = Vec::with_capacity(bfs.folders.len() + bfs.files.len());

    for (path, props) in &bfs.folders {
        if props.visible {
            scripts.extend(get_scripts_from_directory(&window, &**path).await);
        }
    }

    for (path, props) in &bfs.files {
        if props.visible {
            if let Ok(bundle) = ScriptConfigBundle::minimal_from_path(Path::new(path)).await {
                scripts.push(bundle);
            }
        }
    }

    Ok(scripts)
}

#[tauri::command]
pub async fn pick_bot_folder(window: Window) {
    FileDialogBuilder::new().pick_folder(move |path| {
        if let Some(path) = path {
            match BOT_FOLDER_SETTINGS.write() {
                Ok(mut bfs_lock) => {
                    if let Some(bfs) = bfs_lock.as_mut() {
                        bfs.add_folder(&window, path.to_string_lossy().to_string());
                    } else {
                        ccprintln(&window, "Error: BOT_FOLDER_SETTINGS is None");
                    }
                }
                Err(err) => ccprintln!(&window, "Error locking BOT_FOLDER_SETTINGS: {err}"),
            }
        }
    });
}

#[tauri::command]
pub async fn pick_bot_config(window: Window) {
    FileDialogBuilder::new().add_filter("Bot Cfg File", &["cfg"]).pick_file(move |path| {
        if let Some(path) = path {
            match BOT_FOLDER_SETTINGS.write() {
                Ok(mut bfs_lock) => {
                    if let Some(bfs) = bfs_lock.as_mut() {
                        bfs.add_file(&window, path.to_string_lossy().to_string());
                    } else {
                        ccprintln(&window, "BOT_FOLDER_SETTINGS is None");
                    }
                }
                Err(err) => ccprintln!(&window, "Error locking BOT_FOLDER_SETTINGS: {err}"),
            }
        }
    });
}

#[tauri::command]
pub async fn pick_json_file(window: Window) {
    FileDialogBuilder::new().add_filter("JSON File", &["json"]).pick_file(move |path| {
        if let Some(path) = path {
            if let Err(e) = window.emit("json_file_selected", path.to_string_lossy().to_string()) {
                ccprintln!(&window, "Error emiting json_file_selected event: {e}");
            }
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
        ccprintln!(&window, "Error opening path: {e}");
    }
}

#[tauri::command]
pub async fn get_looks(path: String) -> Option<BotLooksConfig> {
    match BotLooksConfig::from_path(&*path).await {
        Ok(looks) => Some(looks),
        Err(_) => None,
    }
}

#[tauri::command]
pub async fn save_looks(window: Window, path: String, config: BotLooksConfig) {
    config.save_to_path(&window, &*path);
}

#[tauri::command]
pub async fn get_match_options() -> Result<MatchOptions, String> {
    let mut mo = MatchOptions::default();
    mo.map_types.extend(custom_maps::find_all(
        &BOT_FOLDER_SETTINGS
            .read()
            .map_err(|err| err.to_string())?
            .as_ref()
            .ok_or("BOT_FOLDER_SETTINGS is None")?
            .folders,
    ));
    Ok(mo)
}

#[tauri::command]
pub async fn get_match_settings(window: Window) -> MatchConfig {
    MatchConfig::load(&window).await
}

#[tauri::command]
pub async fn save_match_settings(window: Window, settings: MatchConfig) {
    settings.cleaned_scripts().save_config(&window).await;
}

#[tauri::command]
pub async fn get_team_settings(window: Window) -> HashMap<String, Vec<BotConfigBundle>> {
    let config = load_gui_config(&window).await;
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
    let mut config = load_gui_config(&window).await;
    config.set("team_settings", "blue_team", Some(serde_json::to_string(&clean(&blue_team)).unwrap()));
    config.set("team_settings", "orange_team", Some(serde_json::to_string(&clean(&orange_team)).unwrap()));

    if let Err(e) = config.write(get_config_path()) {
        ccprintln!(&window, "Error saving team settings: {e}");
    }
}

#[tauri::command]
pub async fn get_language_support() -> Result<HashMap<String, bool>, String> {
    let mut lang_support = HashMap::new();

    lang_support.insert("java".to_owned(), get_command_status("java", ["-version"]));
    lang_support.insert("node".to_owned(), get_command_status("node", ["--version"]));
    lang_support.insert("chrome".to_owned(), has_chrome());
    lang_support.insert(
        "fullpython".to_owned(),
        get_command_status(&*PYTHON_PATH.read().map_err(|err| err.to_string())?, ["-c", "import tkinter"]),
    );
    lang_support.insert("dotnet".to_owned(), get_command_status("dotnet", ["--list"]));

    Ok(dbg!(lang_support))
}

#[tauri::command]
pub async fn get_detected_python_path() -> Option<(String, bool)> {
    auto_detect_python()
}

#[tauri::command]
pub async fn get_python_path() -> Result<String, String> {
    Ok(PYTHON_PATH.read().map_err(|err| err.to_string())?.to_owned())
}

#[tauri::command]
pub async fn set_python_path(window: Window, path: String) -> Result<(), String> {
    *PYTHON_PATH.write().map_err(|err| err.to_string())? = path.clone();
    let mut config = load_gui_config(&window).await;
    config.set("python_config", "path", Some(path));

    if let Err(e) = config.write(get_config_path()) {
        ccprintln!(&window, "Error saving python path: {e}");
    }

    Ok(())
}

#[tauri::command]
pub async fn pick_appearance_file(window: Window) {
    FileDialogBuilder::new().add_filter("Appearance Cfg File", &["cfg"]).pick_file(move |path| {
        if let Some(path) = path {
            if let Err(e) = window.emit("set_appearance_file", path.to_string_lossy().to_string()) {
                ccprintln!(&window, "Error setting appearance file: {e}");
            }
        }
    });
}

fn read_recommendations_json<P: AsRef<Path>>(path: P) -> Result<AllRecommendations<String>, String> {
    let raw_json = read_to_string(&path).map_err(|e| format!("Failed to read {e}"))?;

    serde_json::from_str(&raw_json).map_err(|e| format!("Error parsing file {}: {e}", path.as_ref().to_string_lossy()))
}

fn get_recommendations_json(window: &Window, bfs: &BotFolders) -> Option<AllRecommendations<String>> {
    // Search for and load the json file
    for path in bfs.folders.keys() {
        match glob(&format!("{path}/**/recommendations.json")) {
            Ok(pattern) => {
                for path2 in pattern.flatten() {
                    match read_recommendations_json(path2) {
                        Ok(recommendations) => return Some(recommendations),
                        Err(e) => ccprintln(window, e),
                    }
                }
            }
            Err(e) => ccprintln(window, e.to_string()),
        }
    }

    None
}

#[tauri::command]
pub async fn get_recommendations(window: Window) -> Option<AllRecommendations<BotConfigBundle>> {
    let bfs_lock = BOT_FOLDER_SETTINGS.read().ok()?;
    let bfs = bfs_lock.as_ref()?;

    // If we found the json, return the corresponding BotConfigBundles for the bots
    get_recommendations_json(&window, bfs).map(|j| {
        // Get a list of all the bots in (bot name, bot config file path) pairs
        let name_path_pairs = {
            let mut bots = Vec::new();

            bots.extend(
                bfs.folders
                    .iter()
                    .filter_map(|(path, props)| {
                        if props.visible {
                            match glob(&format!("{path}/**/*.cfg")) {
                                Ok(paths) => Some(paths.flatten().filter_map(|path| BotConfigBundle::name_from_path(path.as_path()).ok()).collect::<Vec<_>>()),
                                Err(e) => {
                                    ccprintln(&window, e.to_string());
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    })
                    .flatten(),
            );

            bots.extend(
                bfs.files
                    .iter()
                    .filter_map(|(path, props)| if props.visible { BotConfigBundle::name_from_path(Path::new(path)).ok() } else { None }),
            );

            bots
        };

        let has_rlbot = check_has_rlbot().unwrap_or_default();

        // Load all of the bot config bundles
        j.change_generic(&|bot_name| {
            for (name, path) in &name_path_pairs {
                if name == bot_name {
                    if let Ok(mut bundle) = BotConfigBundle::minimal_from_path_sync(Path::new(path)) {
                        bundle.logo = bundle.load_logo();

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

#[tauri::command]
pub async fn story_load_save(window: Window) -> Option<StoryState> {
    serde_json::from_str(&load_gui_config(&window).await.get("story_mode", "save_state")?).ok()
}

#[tauri::command]
pub async fn story_new_save(window: Window, team_settings: StoryTeamConfig, story_settings: StoryConfig) -> StoryState {
    let state = StoryState::new(team_settings, story_settings);
    state.save(&window);
    state
}

#[tauri::command]
pub async fn story_save_state(window: Window, story_state: Option<StoryState>) {
    if let Some(story_state) = story_state {
        story_state.save(&window);
    }
}

#[tauri::command]
pub async fn story_delete_save(window: Window) {
    let mut conf = load_gui_config(&window).await;
    conf.set("story_mode", "save_state", None);

    if let Err(e) = conf.write(get_config_path()) {
        ccprintln!(&window, "Error writing config: {e}");
    }
}

#[tauri::command]
pub async fn get_map_pack_revision(window: Window) -> Option<String> {
    let location = Path::new(&get_content_folder()).join(MAPPACK_FOLDER);
    let updater = MapPackUpdater::new(location, MAPPACK_REPO.0.to_owned(), MAPPACK_REPO.1.to_owned());
    let index = updater.get_map_index(&window);
    if let Some(index) = index {
        if let Some(revision) = index.get("revision") {
            return Some(revision.to_string());
        }
    }

    None
}

async fn get_custom_story_json(story_settings: &StoryConfig) -> Option<StoryModeConfig> {
    if story_settings.story_id != StoryIDs::Custom {
        return None;
    }

    if let Some(json) = STORIES_CACHE.read().await.get(story_settings) {
        return Some(json.clone());
    }

    let story_config: StoryModeConfig = serde_json::from_str(&read_to_string_async(&story_settings.custom_config.story_path).await.ok()?).ok()?;
    STORIES_CACHE.write().await.insert(story_settings.clone(), story_config.clone());
    Some(story_config)
}

async fn get_story_config(story_settings: &StoryConfig) -> Option<StoryModeConfig> {
    match story_settings.story_id {
        StoryIDs::Default => {
            if story_settings.use_custom_maps {
                Some(stories::cmaps::default::json())
            } else {
                Some(stories::default::json())
            }
        }
        StoryIDs::Easy => {
            if story_settings.use_custom_maps {
                Some(stories::cmaps::easy::json())
            } else {
                Some(stories::easy::json())
            }
        }
        StoryIDs::Custom => get_custom_story_json(story_settings).await,
    }
}

#[tauri::command]
pub async fn get_story_settings(story_settings: StoryConfig) -> HashMap<String, serde_json::Value> {
    get_story_config(&story_settings).await.unwrap_or_default().settings
}

pub async fn get_cities(story_settings: &StoryConfig) -> HashMap<String, City> {
    get_story_config(story_settings).await.unwrap_or_default().cities
}

#[tauri::command]
pub async fn get_cities_json(story_settings: StoryConfig) -> HashMap<String, City> {
    get_cities(&story_settings).await
}

pub async fn get_all_bot_configs(story_settings: &StoryConfig) -> HashMap<String, Bot> {
    let mut bots = bots_base::json().bots;

    if let Some(config) = get_story_config(story_settings).await {
        bots.extend(config.bots);
    }

    bots
}

pub async fn get_all_script_configs(story_settings: &StoryConfig) -> HashMap<String, Script> {
    get_story_config(story_settings).await.unwrap_or_default().scripts
}

// Get the base bots config and merge it with the bots in the story config
#[tauri::command]
pub async fn get_bots_configs(story_settings: StoryConfig) -> HashMap<String, Bot> {
    get_all_bot_configs(&story_settings).await
}
