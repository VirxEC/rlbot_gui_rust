use crate::{
    bot_management::{
        cfg_helper::{self, save_cfg},
        downloader::MapPackUpdater,
    },
    custom_maps,
    rlbot::{
        agents::runnable::Runnable,
        parsing::{
            agent_config_parser::BotLooksConfig,
            bot_config_bundle::{BotConfigBundle, RLBotCfgParseError, ScriptConfigBundle},
            directory_scanner::{scan_directory_for_bot_configs, scan_directory_for_script_configs},
            match_settings_config_parser::MatchOptions,
        },
    },
    settings::*,
    stories::{bots_base, Bot, City, Script, Settings, StoryModeConfig},
    *,
};
use configparser::ini::Ini;
use futures_util::future::join_all;
use glob::glob;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{create_dir_all, read_to_string},
    io,
    path::Path,
};
use tauri::{api::dialog::FileDialogBuilder, async_runtime::block_on as tauri_block_on, Window};
use tokio::fs as async_fs;

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
    GuiTabCategory::default().save_to_config(conf);
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

    if config_path.exists() {
        match async_fs::read_to_string(config_path).await {
            Ok(s) => {
                if let Err(e) = conf.read(s) {
                    ccprintln!(window, "Error reading config file: {e}");
                }
            }
            Err(e) => ccprintln!(window, "Error reading config file: {e}"),
        }
    } else {
        if let Err(e) = create_dir_all(config_path.parent().unwrap()) {
            ccprintln!(window, "Error creating config directory: {e}");
        }

        set_gui_config_to_default(&mut conf);

        if let Err(e) = save_cfg(&conf, config_path).await {
            ccprintln!(window, "Error writing config file: {e}");
        }
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

#[derive(Debug, Error)]
pub enum SaveFolderSettingsError {
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl_serialize_from_display!(SaveFolderSettingsError);

#[tauri::command]
pub async fn save_folder_settings(window: Window, bot_folder_settings: BotFolders) -> Result<(), SaveFolderSettingsError> {
    BOT_FOLDER_SETTINGS.write().await.update_config(&window, bot_folder_settings).map_err(Into::into)
}

#[tauri::command]
pub async fn get_folder_settings() -> BotFolders {
    BOT_FOLDER_SETTINGS.read().await.clone()
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
pub async fn scan_for_bots(window: Window) -> Vec<BotConfigBundle> {
    let bfs = BOT_FOLDER_SETTINGS.read().await;
    let mut bots = Vec::new();

    for (path, _) in bfs.folders.iter().filter(|(_, props)| props.visible) {
        bots.extend(get_bots_from_directory(&window, path).await);
    }

    for (path, _) in bfs.files.iter().filter(|(_, props)| props.visible) {
        if let Ok(bundle) = BotConfigBundle::minimal_from_path(Path::new(path)).await {
            bots.push(bundle);
        }
    }

    bots
}

async fn get_scripts_from_directory(window: &Window, path: &str) -> Vec<ScriptConfigBundle> {
    filter_hidden_bundles(scan_directory_for_script_configs(window, path).await)
}

#[tauri::command]
pub async fn scan_for_scripts(window: Window) -> Vec<ScriptConfigBundle> {
    let bfs = BOT_FOLDER_SETTINGS.read().await.clone();
    let mut scripts = Vec::new();

    for (path, _) in bfs.folders.iter().filter(|(_, props)| props.visible) {
        scripts.extend(get_scripts_from_directory(&window, path).await);
    }

    for (path, _) in bfs.files.iter().filter(|(_, props)| props.visible) {
        if let Ok(bundle) = ScriptConfigBundle::minimal_from_path(Path::new(path)).await {
            scripts.push(bundle);
        }
    }

    scripts
}

#[tauri::command]
pub fn pick_bot_folder(window: Window) {
    FileDialogBuilder::new().pick_folder(move |path| {
        let Some(path) = path else {
            return;
        };

        if let Err(error) = tauri_block_on(BOT_FOLDER_SETTINGS.write()).add_folder(&window, path.to_string_lossy().to_string()) {
            ccprintln!(&window, "Error adding folder: {error}");
        }
    });
}

#[tauri::command]
pub fn pick_bot_config(window: Window) {
    FileDialogBuilder::new().add_filter("Bot Cfg File", &["cfg"]).pick_file(move |path| {
        let Some(path) = path else {
            return;
        };

        if let Err(error) = tauri_block_on(BOT_FOLDER_SETTINGS.write()).add_file(&window, path.to_string_lossy().to_string()) {
            ccprintln!(&window, "Error adding file: {error}");
        }
    });
}

#[tauri::command]
pub fn pick_json_file(window: Window) {
    FileDialogBuilder::new().add_filter("JSON File", &["json"]).pick_file(move |path| {
        let Some(path) = path else {
            return;
        };

        if let Err(e) = window.emit("json_file_selected", path.to_string_lossy().to_string()) {
            ccprintln!(&window, "Error emiting json_file_selected event: {e}");
        }
    });
}

#[derive(Debug, Error)]
pub enum ShowPathInExplorerError {
    #[error("Path does not exist")]
    DoesNotExist,
    #[error("Couldn't open path: {0}")]
    Other(#[from] std::io::Error),
}

impl_serialize_from_display!(ShowPathInExplorerError);

#[tauri::command]
pub fn show_path_in_explorer(mut path: String) -> Result<(), ShowPathInExplorerError> {
    let ppath = Path::new(&*path);

    if !ppath.exists() {
        return Err(ShowPathInExplorerError::DoesNotExist);
    }

    if ppath.is_file() {
        path = ppath.parent().unwrap().to_string_lossy().to_string();
    }

    open::that(path)?;

    Ok(())
}

#[tauri::command]
pub async fn get_looks(path: String) -> Result<BotLooksConfig, cfg_helper::Error> {
    BotLooksConfig::from_path(&path).await
}

#[tauri::command]
pub async fn save_looks(window: Window, path: String, config: BotLooksConfig) {
    config.save_to_path(&window, &path).await;
}

#[tauri::command]
pub async fn get_match_options() -> Result<MatchOptions, String> {
    let mut mo = MatchOptions::default();
    mo.map_types.extend(custom_maps::find_all(&BOT_FOLDER_SETTINGS.read().await.folders));
    Ok(mo)
}

#[tauri::command]
pub async fn get_match_settings(window: Window) -> MatchConfig {
    MatchConfig::load(&window).await
}

#[tauri::command]
pub async fn save_match_settings(window: Window, settings: MatchConfig) {
    settings.save_config(&window).await;
}

async fn trimmed_to_bundle((skill, path): (Option<f32>, String)) -> Result<BotConfigBundle, RLBotCfgParseError> {
    if path == "human" {
        Ok(BotConfigBundle::new_human())
    } else if let Some(skill) = skill {
        Ok(BotConfigBundle::new_psyonix(skill))
    } else {
        BotConfigBundle::minimal_from_path(path).await
    }
}

async fn trimmed_to_bot_bundles(window: &Window, trimmed_bundles: Vec<(Option<f32>, String)>) -> Vec<BotConfigBundle> {
    join_all(trimmed_bundles.into_iter().map(trimmed_to_bundle))
        .await
        .into_iter()
        .flat_map(|f| {
            if let Err(e) = &f {
                ccprintln!(window, "Error loading bot config: {e}");
            }

            f
        })
        .collect()
}

#[tauri::command]
pub async fn get_team_settings(window: Window) -> HashMap<String, Vec<BotConfigBundle>> {
    let config = load_gui_config(&window).await;

    let blue_team = trimmed_to_bot_bundles(
        &window,
        serde_json::from_str(
            &config
                .get("team_settings", "blue_team")
                .unwrap_or_else(|| format!("[{}]", serde_json::to_string(&BotConfigBundle::new_human()).unwrap())),
        )
        .unwrap_or_default(),
    )
    .await;

    let orange_team = trimmed_to_bot_bundles(
        &window,
        serde_json::from_str(&config.get("team_settings", "orange_team").unwrap_or_else(|| "[]".to_owned())).unwrap_or_default(),
    )
    .await;

    let mut bots = HashMap::new();
    bots.insert("blue_team".to_owned(), blue_team);
    bots.insert("orange_team".to_owned(), orange_team);

    bots
}

fn trim_bot_bundles(bundles: Vec<BotConfigBundle>) -> Vec<(Option<f32>, String)> {
    bundles
        .into_iter()
        .map(|b| if b.path.is_empty() { (b.skill, b.runnable_type) } else { (b.skill, b.path) })
        .collect()
}

#[tauri::command]
pub async fn save_team_settings(window: Window, blue_team: Vec<BotConfigBundle>, orange_team: Vec<BotConfigBundle>) {
    let mut config = load_gui_config(&window).await;
    config.set("team_settings", "blue_team", Some(serde_json::to_string(&trim_bot_bundles(blue_team)).unwrap()));
    config.set("team_settings", "orange_team", Some(serde_json::to_string(&trim_bot_bundles(orange_team)).unwrap()));

    if let Err(e) = save_cfg(&config, get_config_path()).await {
        ccprintln!(&window, "Error saving team settings: {e}");
    }
}

#[tauri::command]
pub async fn get_language_support() -> HashMap<String, bool> {
    let mut lang_support = HashMap::new();

    lang_support.insert("java".to_owned(), get_command_status("java", ["-version"]));
    lang_support.insert("node".to_owned(), get_command_status("node", ["--version"]));
    lang_support.insert("chrome".to_owned(), has_chrome());
    lang_support.insert("fullpython".to_owned(), get_command_status(&*PYTHON_PATH.read().await, ["-c", "import tkinter"]));
    lang_support.insert("dotnet".to_owned(), get_command_status("dotnet", ["--list"]));

    dbg!(lang_support)
}

#[tauri::command]
pub async fn get_detected_python_path() -> Option<(String, bool)> {
    auto_detect_python()
}

#[tauri::command]
pub async fn get_python_path() -> String {
    PYTHON_PATH.read().await.to_owned()
}

#[tauri::command]
pub async fn set_python_path(window: Window, path: String) {
    *PYTHON_PATH.write().await = path.clone();
    let mut config = load_gui_config(&window).await;
    config.set("python_config", "path", Some(path));

    if let Err(e) = save_cfg(&config, get_config_path()).await {
        ccprintln!(&window, "Error saving python path: {e}");
    }
}

#[tauri::command]
pub fn pick_appearance_file(window: Window) {
    FileDialogBuilder::new().add_filter("Appearance Cfg File", &["cfg"]).pick_file(move |path| {
        if let Some(path) = path {
            if let Err(e) = window.emit("set_appearance_file", path.to_string_lossy().to_string()) {
                ccprintln!(&window, "Error setting appearance file: {e}");
            }
        }
    });
}

#[derive(Debug, Error)]
enum ReadRecommendationsError {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse file: {0}")]
    Parse(#[from] serde_json::Error),
}

fn read_recommendations_json<P: AsRef<Path>>(path: P) -> Result<AllRecommendations<String>, ReadRecommendationsError> {
    Ok(serde_json::from_str(&read_to_string(&path)?)?)
}

fn get_recommendations_json(window: &Window, bfs: &BotFolders) -> Option<AllRecommendations<String>> {
    bfs.folders
        .keys()
        .filter_map(|path| match glob(&format!("{path}/**/recommendations.json")) {
            Ok(pattern) => Some(pattern),
            Err(e) => {
                ccprintln!(window, "{e}");
                None
            }
        })
        .flatten()
        .flatten()
        .find_map(|path| match read_recommendations_json(path) {
            Ok(recommendations) => Some(recommendations),
            Err(e) => {
                ccprintln(window, e.to_string());
                None
            }
        })
}

#[tauri::command]
pub async fn get_recommendations(window: Window) -> Option<AllRecommendations<BotConfigBundle>> {
    let bfs = BOT_FOLDER_SETTINGS.read().await.clone();
    let python_path = PYTHON_PATH.read().await.to_owned();
    let has_rlbot = check_has_rlbot().await;

    // If we found the json, return the corresponding BotConfigBundles for the bots
    get_recommendations_json(&window, &bfs).map(|j| {
        let folders = bfs
            .folders
            .iter()
            .filter(|(_, props)| props.visible)
            .filter_map(|(path, _)| match glob(&format!("{path}/**/*.cfg")) {
                Ok(paths) => Some(paths.flatten().filter_map(|path| BotConfigBundle::name_from_path(path.as_path()).ok()).collect::<Vec<_>>()),
                Err(e) => {
                    ccprintln(&window, e.to_string());
                    None
                }
            })
            .flatten();

        let files = bfs
            .files
            .iter()
            .filter(|(_, props)| props.visible)
            .filter_map(|(path, _)| BotConfigBundle::name_from_path(Path::new(path)).ok());

        // Get a list of all the bots in (bot name, bot config file path) pairs
        let name_path_pairs = folders.chain(files).collect::<Vec<_>>();

        // Load all of the bot config bundles
        j.change_generic(&|bot_name| {
            name_path_pairs
                .iter()
                .filter(|(name, _)| name == bot_name)
                .find_map(|(_, path)| BotConfigBundle::minimal_from_path_sync(Path::new(path)).ok())
                .map(|mut bundle| {
                    bundle.logo = bundle.load_logo();

                    if has_rlbot {
                        let missing_packages = bundle.get_missing_packages(&window, &python_path);
                        if !missing_packages.is_empty() {
                            bundle.warn = Some("pythonpkg".to_owned());
                        }
                        bundle.missing_python_packages = Some(missing_packages);
                    }

                    bundle
                })
                .unwrap_or_default()
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
    state.save(&window).await;
    state
}

#[tauri::command]
pub async fn story_save_state(window: Window, story_state: Option<StoryState>) {
    if let Some(story_state) = story_state {
        story_state.save(&window).await;
    }
}

#[tauri::command]
pub async fn story_delete_save(window: Window) {
    let mut conf = load_gui_config(&window).await;
    conf.set("story_mode", "save_state", None);

    if let Err(e) = save_cfg(&conf, get_config_path()).await {
        ccprintln!(&window, "Error writing config: {e}");
    }
}

#[tauri::command]
pub async fn get_map_pack_revision(window: Window) -> Option<String> {
    let location = Path::new(&get_content_folder()).join(MAPPACK_FOLDER);
    let updater = MapPackUpdater::new(location, MAPPACK_REPO.0.to_owned(), MAPPACK_REPO.1.to_owned());

    Some(updater.get_map_index(&window).await?.get("revision")?.to_string())
}

async fn get_custom_story_json(story_settings: &StoryConfig) -> Option<StoryModeConfig> {
    if story_settings.story_id != StoryIDs::Custom {
        return None;
    }

    if let Some(json) = STORIES_CACHE.read().await.get(story_settings) {
        return Some(json.clone());
    }

    let story_config: StoryModeConfig = serde_json::from_str(&async_fs::read_to_string(&story_settings.custom_config.story_path).await.ok()?).ok()?;
    STORIES_CACHE.write().await.insert(story_settings.clone(), story_config.clone());
    Some(story_config)
}

async fn get_story_config(story_settings: &StoryConfig) -> Option<StoryModeConfig> {
    match story_settings.story_id {
        StoryIDs::Default if story_settings.use_custom_maps => Some(stories::cmaps::default::JSON.clone()),
        StoryIDs::Default => Some(stories::default::JSON.clone()),
        StoryIDs::Easy if story_settings.use_custom_maps => Some(stories::cmaps::easy::JSON.clone()),
        StoryIDs::Easy => Some(stories::easy::JSON.clone()),
        StoryIDs::Custom => get_custom_story_json(story_settings).await,
    }
}

#[tauri::command]
pub async fn get_story_settings(story_settings: StoryConfig) -> Settings {
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
    let mut bots = bots_base::JSON.bots.clone();

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrimaryCategories {
    #[default]
    All,
    Standard,
    Extra,
    Special,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuiTabCategory {
    primary: PrimaryCategories,
    secondary: usize,
}

impl GuiTabCategory {
    pub fn new(primary: PrimaryCategories, secondary: usize) -> Self {
        Self { primary, secondary }
    }

    async fn load_primary(window: &Window) -> Option<PrimaryCategories> {
        serde_json::from_str(&load_gui_config(window).await.get("gui_state", "selected_tab")?).ok()
    }

    async fn load_seconary(window: &Window) -> Option<usize> {
        serde_json::from_str(&load_gui_config(window).await.get("gui_state", "selected_tab_secondary")?).ok()
    }

    pub async fn load(window: &Window) -> Self {
        Self {
            primary: Self::load_primary(window).await.unwrap_or_default(),
            secondary: Self::load_seconary(window).await.unwrap_or_default(),
        }
    }

    pub fn save_to_config(&self, conf: &mut Ini) {
        conf.set("gui_state", "selected_tab", Some(serde_json::to_string(&self.primary).unwrap()));
        conf.set("gui_state", "selected_tab_secondary", Some(serde_json::to_string(&self.secondary).unwrap()));
    }

    pub async fn save(&self, window: &Window) {
        let mut conf = load_gui_config(window).await;
        self.save_to_config(&mut conf);

        if let Err(e) = save_cfg(&conf, get_config_path()).await {
            ccprintln!(window, "Error writing config: {e}");
        }
    }
}

#[tauri::command]
pub async fn get_selected_tab(window: Window) -> GuiTabCategory {
    GuiTabCategory::load(&window).await
}

#[tauri::command]
pub async fn set_selected_tab(window: Window, primary: PrimaryCategories, secondary: usize) {
    GuiTabCategory::new(primary, secondary).save(&window).await;
}
