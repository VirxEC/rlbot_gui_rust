use crate::bot_management::{
    bot_creation::{bootstrap_python_bot, bootstrap_python_hivemind, bootstrap_rust_bot, bootstrap_scratch_bot, CREATED_BOTS_FOLDER},
    downloader,
};
use crate::custom_maps::find_all_custom_maps;
use crate::rlbot::{agents::runnable::Runnable, parsing::match_settings_config_parser::*};
use crate::rlbot::{
    gateway_util,
    parsing::{
        agent_config_parser::BotLooksConfig,
        bot_config_bundle::{BotConfigBundle, ScriptConfigBundle},
        directory_scanner::{scan_directory_for_bot_configs, scan_directory_for_script_configs},
    },
    setup_manager,
};
use crate::settings::*;
use crate::*;
use glob::glob;
use native_dialog::FileDialog;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelExtend, ParallelIterator};
use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir_all, read_to_string, File},
    io::{copy, Cursor},
    path::Path,
    process::{Command, Stdio},
};
use tauri::Window;

#[tauri::command]
pub async fn save_folder_settings(bot_folder_settings: BotFolderSettings) {
    BOT_FOLDER_SETTINGS.lock().unwrap().update_config(bot_folder_settings)
}

#[tauri::command]
pub async fn get_folder_settings() -> BotFolderSettings {
    BOT_FOLDER_SETTINGS.lock().unwrap().clone()
}

fn filter_hidden_bundles<T: Runnable + Clone>(bundles: HashSet<T>) -> Vec<T> {
    bundles.iter().filter(|b| !b.get_config_file_name().starts_with('_')).cloned().collect()
}

fn get_bots_from_directory(path: &str) -> Vec<BotConfigBundle> {
    filter_hidden_bundles(scan_directory_for_bot_configs(path))
}

#[tauri::command]
pub async fn scan_for_bots() -> Vec<BotConfigBundle> {
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
pub async fn scan_for_scripts() -> Vec<ScriptConfigBundle> {
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

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn pick_bot_folder() {
    let path = match FileDialog::new().show_open_single_dir().unwrap() {
        Some(path) => path,
        None => return,
    };

    BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(path.to_str().unwrap().to_string());
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn pick_bot_folder(window: Window) {
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
pub async fn pick_bot_config() {
    let path = match FileDialog::new().add_filter("Bot Cfg File", &["cfg"]).show_open_single_file().unwrap() {
        Some(path) => path,
        None => return,
    };

    BOT_FOLDER_SETTINGS.lock().unwrap().add_file(path.to_str().unwrap().to_string());
}

#[tauri::command]
pub async fn show_path_in_explorer(path: String) {
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
pub async fn get_looks(path: String) -> Option<BotLooksConfig> {
    match BotLooksConfig::from_path(&*path) {
        Ok(looks) => Some(looks),
        Err(_) => None,
    }
}

#[tauri::command]
pub async fn save_looks(path: String, config: BotLooksConfig) {
    config.save_to_path(&*path);
}

#[tauri::command]
pub async fn get_match_options() -> MatchOptions {
    let mut mo = MatchOptions::new();
    mo.map_types.extend(find_all_custom_maps(&BOT_FOLDER_SETTINGS.lock().unwrap().folders));
    mo
}

#[tauri::command]
pub async fn get_match_settings() -> MatchSettings {
    MATCH_SETTINGS.lock().unwrap().clone()
}

#[tauri::command]
pub async fn save_match_settings(settings: MatchSettings) {
    MATCH_SETTINGS.lock().unwrap().update_config(settings.cleaned_scripts());
}

#[tauri::command]
pub async fn get_team_settings() -> HashMap<String, Vec<BotConfigBundle>> {
    let config = load_gui_config();
    let blue_team = serde_json::from_str(
        &config
            .get("team_settings", "blue_team")
            .unwrap_or_else(|| "[{\"name\": \"Human\", \"runnable_type\": \"human\", \"image\": \"imgs/human.png\"}]".to_string()),
    )
    .unwrap_or_default();
    let orange_team = serde_json::from_str(&config.get("team_settings", "orange_team").unwrap_or_else(|| "[]".to_string())).unwrap_or_default();

    let mut bots = HashMap::new();
    bots.insert("blue_team".to_string(), blue_team);
    bots.insert("orange_team".to_string(), orange_team);

    bots
}

#[tauri::command]
pub async fn save_team_settings(blue_team: Vec<BotConfigBundle>, orange_team: Vec<BotConfigBundle>) {
    let mut config = load_gui_config();
    config.set("team_settings", "blue_team", Some(serde_json::to_string(&clean(blue_team)).unwrap()));
    config.set("team_settings", "orange_team", Some(serde_json::to_string(&clean(orange_team)).unwrap()));
    config.write(get_config_path()).unwrap();
}

#[tauri::command]
pub async fn get_language_support() -> HashMap<String, bool> {
    let mut lang_support = HashMap::new();

    lang_support.insert("java".to_string(), get_command_status("java", vec!["-version"]));
    lang_support.insert("node".to_string(), get_command_status("node", vec!["--version"]));
    lang_support.insert("chrome".to_string(), has_chrome());
    lang_support.insert("fullpython".to_string(), get_command_status(&PYTHON_PATH.lock().unwrap(), vec!["-c", "import tkinter"]));

    dbg!(lang_support)
}

#[tauri::command]
pub async fn check_rlbot_python() -> HashMap<String, bool> {
    let mut python_support = HashMap::new();

    let python_path = PYTHON_PATH.lock().unwrap().to_string();

    if get_command_status(&python_path, vec!["--version"]) {
        python_support.insert("python".to_string(), true);
        python_support.insert(
            "rlbotpython".to_string(),
            get_command_status(&python_path, vec!["-c", "import rlbot; import numpy; import numba; import scipy; import selenium"]),
        );
    } else {
        python_support.insert("python".to_string(), false);
        python_support.insert("rlbotpython".to_string(), false);
    }

    dbg!(python_support)
}

#[tauri::command]
pub async fn get_detected_python_path() -> Option<String> {
    auto_detect_python()
}

#[tauri::command]
pub async fn get_python_path() -> String {
    PYTHON_PATH.lock().unwrap().to_string()
}

#[tauri::command]
pub async fn set_python_path(path: String) {
    *PYTHON_PATH.lock().unwrap() = path.clone();
    let mut config = load_gui_config();
    config.set("python_config", "path", Some(path));
    config.write(get_config_path()).unwrap();
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn pick_appearance_file() -> Option<String> {
    match FileDialog::new().add_filter("Appearance Cfg File", &["cfg"]).show_open_single_file() {
        Ok(path) => path.map(|path| path.to_str().unwrap().to_string()),
        Err(e) => {
            ccprintlne(e.to_string());
            None
        }
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn pick_appearance_file(window: Window) -> Option<String> {
    // FileDialog must be ran on the main thread when running on MacOS, it will panic if it isn't
    let out = Arc::new(Mutex::new(None));
    let out_clone = Arc::clone(&out);
    window
        .run_on_main_thread(move || {
            let mut out_ref = out_clone.lock().unwrap();
            *out_ref = match FileDialog::new().add_filter("Appearance Cfg File", &["cfg"]).show_open_single_file() {
                Ok(path) => path.map(|path| path.to_str().unwrap().to_string()),
                Err(e) => {
                    ccprintlne(e.to_string());
                    None
                }
            };
        })
        .unwrap();

    // Rust requries that we first store the clone in a variable before we return it so out can be dropped safely
    let x = out.lock().unwrap().clone();
    x
}

fn get_recommendations_json() -> Option<AllRecommendations<String>> {
    // Search for and load the json file
    for path in BOT_FOLDER_SETTINGS.lock().unwrap().folders.keys() {
        let pattern = Path::new(path).join("**/recommendations.json");

        for path2 in glob(pattern.to_str().unwrap()).unwrap().flatten() {
            let raw_json = match read_to_string(&path2) {
                Ok(s) => s,
                Err(_) => {
                    ccprintlne(format!("Failed to read {}", path2.to_str().unwrap()));
                    continue;
                }
            };

            match serde_json::from_str(&raw_json) {
                Ok(j) => return Some(j),
                Err(e) => {
                    ccprintlne(format!("Failed to parse file {}: {}", path2.to_str().unwrap(), e));
                    continue;
                }
            }
        }
    }

    None
}

#[tauri::command]
pub async fn get_recommendations() -> Option<AllRecommendations<BotConfigBundle>> {
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
        j.change_generic(&|bot_name| {
            for (name, path) in &name_path_pairs {
                if name == bot_name {
                    if let Ok(mut bundle) = BotConfigBundle::minimal_from_path(Path::new(path)) {
                        bundle.logo = bundle.get_logo();

                        if has_rlbot {
                            let missing_packages = bundle.get_missing_packages();
                            if !missing_packages.is_empty() {
                                bundle.warn = Some("pythonpkg".to_string());
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

fn ensure_bot_directory() -> String {
    let bot_directory = get_content_folder();
    let bot_directory_path = Path::new(&bot_directory).join(CREATED_BOTS_FOLDER);

    if !bot_directory_path.exists() {
        create_dir_all(&bot_directory_path).unwrap();
    }

    bot_directory.to_str().unwrap().to_string()
}

#[tauri::command]
pub async fn begin_python_bot(bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_python_bot(bot_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[tauri::command]
pub async fn begin_python_hivemind(hive_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_python_hivemind(hive_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[tauri::command]
pub async fn begin_rust_bot(bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_rust_bot(bot_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[tauri::command]
pub async fn begin_scratch_bot(bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_scratch_bot(bot_name, &ensure_bot_directory()).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_string(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_string(), e)])),
    }
}

#[tauri::command]
pub async fn install_package(package_string: String) -> PackageResult {
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
pub async fn install_requirements(config_path: String) -> PackageResult {
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
pub async fn install_basic_packages() -> PackageResult {
    install_upgrade_basic_packages()
}

#[tauri::command]
pub async fn get_console_texts() -> Vec<ConsoleText> {
    CONSOLE_TEXT.lock().unwrap().clone()
}

#[tauri::command]
pub async fn get_missing_bot_packages(bots: Vec<BotConfigBundle>) -> Vec<MissingPackagesUpdate> {
    if check_has_rlbot() {
        bots.par_iter()
            .enumerate()
            .filter_map(|(index, bot)| {
                if bot.runnable_type == *"rlbot" {
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
                if bot.runnable_type == *"rlbot" && (bot.warn.is_some() || bot.missing_python_packages.is_some()) {
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
pub async fn get_missing_script_packages(scripts: Vec<ScriptConfigBundle>) -> Vec<MissingPackagesUpdate> {
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

#[tauri::command]
pub async fn get_missing_bot_logos(bots: Vec<BotConfigBundle>) -> Vec<LogoUpdate> {
    bots.par_iter()
        .enumerate()
        .filter_map(|(index, bot)| {
            if bot.runnable_type == *"rlbot" && bot.logo.is_none() {
                if let Some(logo) = bot.get_logo() {
                    return Some(LogoUpdate { index, logo });
                }
            }

            None
        })
        .collect()
}

#[tauri::command]
pub async fn get_missing_script_logos(scripts: Vec<ScriptConfigBundle>) -> Vec<LogoUpdate> {
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

#[tauri::command]
pub fn is_windows() -> bool {
    cfg!(windows)
}

#[tauri::command]
pub async fn install_python() -> Option<u8> {
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
pub async fn download_bot_pack(window: Window) -> String {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_str().unwrap().to_string();
    let botpack_status = downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await;

    match botpack_status {
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(botpack_location);
            message
        }
        downloader::BotpackStatus::Skipped(message) => message,
        _ => unreachable!(),
    }
}

#[tauri::command]
pub async fn update_bot_pack(window: Window) -> String {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_str().unwrap().to_string();
    let botpack_status = downloader::update_bot_pack(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location).await;

    match botpack_status {
        downloader::BotpackStatus::Skipped(message) => message,
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(botpack_location);
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(botpack_location);
                    message
                }
                downloader::BotpackStatus::Skipped(message) => message,
                _ => unreachable!(),
            }
        }
    }
}

#[tauri::command]
pub async fn update_map_pack(window: Window) -> String {
    let mappack_location = get_content_folder().join(MAPPACK_FOLDER);
    let updater = downloader::MapPackUpdater::new(&mappack_location, MAPPACK_REPO.0.to_string(), MAPPACK_REPO.1.to_string());
    let location = mappack_location.to_str().unwrap();
    let map_index_old = updater.get_map_index();

    match updater.needs_update().await {
        downloader::BotpackStatus::Skipped(message) => message,
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(location.to_string());
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, MAPPACK_REPO.0, MAPPACK_REPO.1, location, false).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.lock().unwrap().add_folder(mappack_location.to_str().unwrap().to_string());

                    if updater.get_map_index().is_none() {
                        ccprintlne("Couldn't find revision number in map pack".to_string());
                        return "Couldn't find revision number in map pack".to_string();
                    }

                    updater.hydrate_map_pack(map_index_old).await;

                    message
                }
                downloader::BotpackStatus::Skipped(message) => message,
                _ => unreachable!(),
            }
        }
    }
}

#[tauri::command]
pub async fn is_botpack_up_to_date() -> bool {
    let repo_full_name = format!("{}/{}", BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME);
    bot_management::downloader::is_botpack_up_to_date(&repo_full_name).await
}

#[tauri::command]
pub async fn get_launcher_settings() -> LauncherSettings {
    LauncherSettings::new()
}

#[tauri::command]
pub async fn save_launcher_settings(settings: LauncherSettings) {
    settings.write_to_file();
}

fn start_match_process(bot_list: &str, match_settings: &str, preferred_launcher: &str, use_login_tricks: bool, rocket_league_exe_path: Option<String>) {
    let mut command = Command::new(&*PYTHON_PATH.lock().unwrap());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // disable window creation
        command.creation_flags(0x08000000);
    };

    let script_path = get_content_folder().join("start_match.py");
    let args = vec![
        script_path.to_str().unwrap().to_string(),
        format!("bot_list={}", bot_list),
        format!("match_settings={}", match_settings),
        format!("preferred_launcher={}", preferred_launcher),
        format!("use_login_tricks={}", use_login_tricks),
        format!("rocket_league_exe_path={}", rocket_league_exe_path.unwrap_or_else(|| "None".to_string())),
    ];

    let mut child = if let Ok(the_child) = command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        the_child
    } else {
        return;
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

    child.wait().unwrap();
    STDOUT_CAPTURE.lock().unwrap()[stdout_index] = None;
    STDERR_CAPTURE.lock().unwrap()[stderr_index] = None;
}

#[tauri::command]
pub async fn start_match(bot_list: Vec<TeamBotBundle>, match_settings: MatchSettings) -> bool {
    let port = gateway_util::find_existing_process().unwrap_or(gateway_util::IDEAL_RLBOT_PORT);
    dbg!(port);

    match setup_manager::is_rocket_league_running(port) {
        Ok(is_running) => ccprintln(format!(
            "Rocket League is {}",
            if is_running { "already running with RLBot args!" } else { "not running yet..." }
        )),
        Err(err) => {
            ccprintlne(err);
            return false;
        }
    }

    let launcher_settings = LauncherSettings::new();

    // TODO: we can send input, but we can't get output right now
    // this means no error messages :(
    // possible solution would be to just monitor stderr and fire an event
    // TODO: this process is blocking, a way should be found to make it non-blocking
    start_match_process(
        serde_json::to_string(&bot_list).unwrap().as_str(),
        serde_json::to_string(&match_settings.super_cleaned_scripts()).unwrap().as_str(),
        launcher_settings.preferred_launcher.as_str(),
        launcher_settings.use_login_tricks,
        launcher_settings.rocket_league_exe_path,
    );

    true
}
