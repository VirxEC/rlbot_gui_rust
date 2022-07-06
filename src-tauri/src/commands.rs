use crate::{
    bot_management::{
        bot_creation::{bootstrap_python_bot, bootstrap_python_hivemind, bootstrap_rust_bot, bootstrap_scratch_bot, CREATED_BOTS_FOLDER},
        downloader::{self, get_current_tag_name, ProgressBarUpdate},
        zip_extract_fixed,
    },
    rlbot::{
        agents::runnable::Runnable,
        gateway_util,
        parsing::{
            agent_config_parser::BotLooksConfig,
            bot_config_bundle::{BotConfigBundle, ScriptConfigBundle},
        },
        setup_manager,
    },
    settings::*,
    *,
};
use futures_util::StreamExt;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::{copy, Cursor, Write},
    path::Path,
    time::Instant,
};
use tauri::Window;

#[tauri::command]
pub async fn check_rlbot_python() -> HashMap<String, bool> {
    let mut python_support = HashMap::new();

    let python_path = PYTHON_PATH.lock().unwrap().to_owned();

    if get_command_status(&python_path, vec!["--version"]) {
        python_support.insert("python".to_owned(), true);
        python_support.insert(
            "rlbotpython".to_owned(),
            get_command_status(&python_path, vec!["-c", "import rlbot; import numpy; import numba; import scipy; import selenium"]),
        );
    } else {
        python_support.insert("python".to_owned(), false);
        python_support.insert("rlbotpython".to_owned(), false);
    }

    dbg!(python_support)
}

fn ensure_bot_directory(window: &Window) -> PathBuf {
    let bot_directory_path = get_content_folder().join(CREATED_BOTS_FOLDER);

    if !bot_directory_path.exists() {
        if let Err(e) = create_dir_all(&bot_directory_path) {
            ccprintlne(window, format!("Error creating bot directory: {}", e));
        }
    }

    bot_directory_path
}

#[tauri::command]
pub async fn begin_python_bot(window: Window, bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_python_bot(&window, bot_name, ensure_bot_directory(&window)).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_owned(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_owned(), e)])),
    }
}

#[tauri::command]
pub async fn begin_python_hivemind(window: Window, hive_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_python_hivemind(&window, hive_name, ensure_bot_directory(&window)).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_owned(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_owned(), e)])),
    }
}

#[tauri::command]
pub async fn begin_rust_bot(window: Window, bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_rust_bot(&window, bot_name, ensure_bot_directory(&window)).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_owned(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_owned(), e)])),
    }
}

#[tauri::command]
pub async fn begin_scratch_bot(window: Window, bot_name: String) -> Result<HashMap<String, BotConfigBundle>, HashMap<String, String>> {
    match bootstrap_scratch_bot(&window, bot_name, ensure_bot_directory(&window)).await {
        Ok(config_file) => Ok(HashMap::from([("bot".to_owned(), BotConfigBundle::minimal_from_path(Path::new(&config_file)).unwrap())])),
        Err(e) => Err(HashMap::from([("error".to_owned(), e)])),
    }
}

#[tauri::command]
pub async fn install_package(package_string: String) -> PackageResult {
    let exit_code = spawn_capture_process_and_get_exit_code(
        PYTHON_PATH.lock().unwrap().to_owned(),
        &["-m", "pip", "install", "-U", "--no-warn-script-location", &package_string],
    );

    PackageResult {
        exit_code,
        packages: vec![package_string],
    }
}

#[tauri::command]
pub async fn install_requirements(window: Window, config_path: String) -> PackageResult {
    let bundle = BotConfigBundle::minimal_from_path(Path::new(&config_path)).unwrap();

    if let Some(file) = bundle.get_requirements_file() {
        let packages = bundle.get_missing_packages(&window);
        let python = PYTHON_PATH.lock().unwrap().to_owned();
        let exit_code = spawn_capture_process_and_get_exit_code(&python, &["-m", "pip", "install", "-U", "--no-warn-script-location", "-r", file]);

        PackageResult { exit_code, packages }
    } else {
        PackageResult {
            exit_code: 1,
            packages: vec!["unknown file".to_owned()],
        }
    }
}

async fn install_upgrade_basic_packages(window: &Window) -> PackageResult {
    let packages = vec![
        String::from("pip"),
        String::from("setuptools"),
        String::from("wheel"),
        String::from("numpy<1.23"),
        String::from("scipy"),
        String::from("numba<0.56"),
        String::from("selenium"),
        String::from("rlbot"),
    ];

    if !is_online::check().await {
        ccprintlne(
            window,
            "Could not connect to the internet to install/update basic packages. Please check your internet connection and try again.".to_string(),
        );

        return PackageResult { exit_code: 3, packages };
    }

    let python = PYTHON_PATH.lock().unwrap().to_owned();

    spawn_capture_process_and_get_exit_code(&python, &["-m", "ensurepip"]);

    let mut exit_code = 0;

    for package in &packages {
        if exit_code != 0 {
            break;
        }

        exit_code = spawn_capture_process_and_get_exit_code(&python, &["-m", "pip", "install", "-U", "--no-warn-script-location", package]);
    }

    PackageResult { exit_code, packages }
}

#[tauri::command]
pub async fn install_basic_packages(window: Window) -> PackageResult {
    install_upgrade_basic_packages(&window).await
}

#[tauri::command]
pub async fn get_console_texts() -> Vec<ConsoleText> {
    CONSOLE_TEXT.lock().unwrap().clone()
}

#[tauri::command]
pub async fn get_missing_bot_packages(window: Window, bots: Vec<BotConfigBundle>) -> Vec<MissingPackagesUpdate> {
    if check_has_rlbot() {
        bots.par_iter()
            .enumerate()
            .filter_map(|(index, bot)| {
                if bot.runnable_type == *"rlbot" {
                    let mut warn = bot.warn.clone();
                    let mut missing_packages = bot.missing_python_packages.clone();

                    if let Some(missing_packages) = &missing_packages {
                        if warn == Some("pythonpkg".to_owned()) && missing_packages.is_empty() {
                            warn = None;
                        }
                    } else {
                        let bot_missing_packages = bot.get_missing_packages(&window);

                        if !bot_missing_packages.is_empty() {
                            warn = Some("pythonpkg".to_owned());
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
pub async fn get_missing_script_packages(window: Window, scripts: Vec<ScriptConfigBundle>) -> Vec<MissingPackagesUpdate> {
    if check_has_rlbot() {
        scripts
            .par_iter()
            .enumerate()
            .filter_map(|(index, script)| {
                let mut warn = script.warn.clone();
                let mut missing_packages = script.missing_python_packages.clone();

                if let Some(missing_packages) = &missing_packages {
                    if warn == Some("pythonpkg".to_owned()) && missing_packages.is_empty() {
                        warn = None;
                    }
                } else {
                    let script_missing_packages = script.get_missing_packages(&window);

                    if !script_missing_packages.is_empty() {
                        warn = Some("pythonpkg".to_owned());
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
pub async fn install_python(window: Window) -> Option<u8> {
    let content_folder = get_content_folder();
    let folder_destination = content_folder.join("Python37");
    let file_path = content_folder.join("python-3.7.9-custom-amd64.zip");

    let download_url = "https://virxec.github.io/rlbot_gui_rust/python-3.7.9-custom-amd64.zip";
    let res = reqwest::Client::new().get(download_url).send().await.ok()?;
    let total_size = 21_873_000;
    let mut stream = res.bytes_stream();
    let mut bytes = Vec::with_capacity(total_size);
    let mut last_update = Instant::now();

    if !file_path.exists() {
        while let Some(new_bytes) = stream.next().await {
            // put the new bytes into bytes
            bytes.extend_from_slice(&new_bytes.ok()?);

            if last_update.elapsed().as_secs_f32() >= 0.1 {
                let progress = bytes.len() as f32 / total_size as f32 * 100.0;
                if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, "Downloading zip...".to_owned())) {
                    ccprintlne(&window, format!("Error when updating progress bar: {}", e));
                }
                last_update = Instant::now();
            }
        }

        if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(100., "Writing zip to disk...".to_owned())) {
            ccprintlne(&window, format!("Error when updating progress bar: {}", e));
        }

        let mut file = File::create(&file_path).ok()?;
        let mut content = Cursor::new(bytes);
        copy(&mut content, &mut file).ok()?;
    }

    if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(100., "Extracting zip...".to_owned())) {
        ccprintlne(&window, format!("Error when updating progress bar: {}", e));
    }

    // Extract the zip file
    let file = File::open(&file_path).ok()?;
    zip_extract_fixed::extract(&window, &file, folder_destination.as_path(), false, false).ok()?;

    // Updat the Python path
    *PYTHON_PATH.lock().unwrap() = folder_destination.join("python.exe").to_string_lossy().to_string();

    Some(0)
}

#[tauri::command]
pub async fn download_bot_pack(window: Window) -> String {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_string_lossy().to_string();
    let botpack_status = downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await;

    match botpack_status {
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_folder(&window, botpack_location);
            message
        }
        downloader::BotpackStatus::Skipped(message) => message,
        _ => unreachable!(),
    }
}

#[tauri::command]
pub async fn update_bot_pack(window: Window) -> String {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_string_lossy().to_string();
    let botpack_status = downloader::update_bot_pack(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location).await;

    match botpack_status {
        downloader::BotpackStatus::Skipped(message) => message,
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_folder(&window, botpack_location);
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_folder(&window, botpack_location);
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
    let updater = downloader::MapPackUpdater::new(&mappack_location, MAPPACK_REPO.0.to_owned(), MAPPACK_REPO.1.to_owned());
    let location = mappack_location.to_string_lossy().to_string();
    let map_index_old = updater.get_map_index(&window);

    match updater.needs_update(&window).await {
        downloader::BotpackStatus::Skipped(message) => {
            BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_folder(&window, location);
            message
        }
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_folder(&window, location);
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, MAPPACK_REPO.0, MAPPACK_REPO.1, &location, false).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_folder(&window, location);

                    if updater.get_map_index(&window).is_none() {
                        ccprintlne(&window, "Couldn't find revision number in map pack".to_owned());
                        return "Couldn't find revision number in map pack".to_owned();
                    }

                    updater.hydrate_map_pack(&window, map_index_old).await;

                    message
                }
                downloader::BotpackStatus::Skipped(message) => message,
                _ => unreachable!(),
            }
        }
    }
}

#[tauri::command]
pub async fn is_botpack_up_to_date(window: Window) -> bool {
    let repo_full_name = format!("{}/{}", BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME);
    bot_management::downloader::is_botpack_up_to_date(&window, &repo_full_name).await
}

#[tauri::command]
pub async fn get_launcher_settings(window: Window) -> LauncherSettings {
    LauncherSettings::load(&window)
}

#[tauri::command]
pub async fn save_launcher_settings(window: Window, settings: LauncherSettings) {
    settings.write_to_file(&window);
}

fn create_match_handler(window: &Window) -> Option<ChildStdin> {
    let program = PYTHON_PATH.lock().unwrap().clone();
    let script_path = get_content_folder().join("match_handler.py").to_string_lossy().to_string();

    match get_capture_command(program, &[&script_path]).stdin(Stdio::piped()).spawn() {
        Ok(mut child) => child.stdin.take(),
        Err(err) => {
            ccprintlne(window, format!("Failed to start match handler: {}", err));
            None
        }
    }
}

pub fn issue_match_handler_command(window: &Window, command_parts: &[String], create_handler: bool) {
    let mut match_handler_stdin = MATCH_HANDLER_STDIN.lock().unwrap();

    if match_handler_stdin.is_none() {
        if create_handler {
            ccprintln(window, "Starting match handler!".to_owned());
            *match_handler_stdin = create_match_handler(window);
        } else {
            ccprintln(window, "Not issuing command to handler as it's down and I was told to not start it".to_owned());
            return;
        }
    }

    let command = format!("{} | \n", command_parts.join(" | "));
    let stdin = match_handler_stdin.as_mut().unwrap();

    if stdin.write_all(command.as_bytes()).is_err() {
        match_handler_stdin.take().unwrap();
    }
}

#[tauri::command]
pub async fn start_match(window: Window, bot_list: Vec<TeamBotBundle>, match_settings: MatchSettings) -> bool {
    let port = gateway_util::find_existing_process(&window);

    match setup_manager::is_rocket_league_running(port.unwrap_or(gateway_util::IDEAL_RLBOT_PORT)) {
        Ok(rl_is_running) => {
            ccprintln(
                &window,
                format!("Rocket League is {}", if rl_is_running { "already running with RLBot args!" } else { "not running yet..." }),
            );

            // kill RLBot if it's running but Rocket League isn't
            if !rl_is_running && port.is_some() {
                kill_bots(window.clone()).await;
                gateway_util::kill_existing_processes(&window);
            }
        }
        Err(err) => {
            ccprintlne(&window, err);
            return false;
        }
    }

    let launcher_settings = LauncherSettings::load(&window);

    let match_settings = match match_settings.setup_for_start_match(&window, &BOT_FOLDER_SETTINGS.lock().unwrap().as_ref().unwrap().folders) {
        Some(match_settings) => match_settings,
        None => {
            if let Err(e) = window.emit("match-start-failed", ()) {
                ccprintlne(&window, format!("Failed to emit match-start-failed: {}", e));
            }

            return false;
        }
    };

    let args = [
        "start_match".to_owned(),
        serde_json::to_string(&bot_list).unwrap().as_str().to_owned(),
        serde_json::to_string(&match_settings).unwrap().as_str().to_owned(),
        launcher_settings.preferred_launcher,
        launcher_settings.use_login_tricks.to_string(),
        launcher_settings.rocket_league_exe_path.unwrap_or_default(),
    ];

    println!("Issuing command: {} | ", args.join(" | "));

    issue_match_handler_command(&window, &args, true);

    true
}

#[tauri::command]
pub async fn kill_bots(window: Window) {
    issue_match_handler_command(&window, &["shut_down".to_owned()], false);

    let mut match_handler_stdin = MATCH_HANDLER_STDIN.lock().unwrap();
    if match_handler_stdin.is_some() {
        match_handler_stdin.take().unwrap();
    }
}

#[tauri::command]
pub async fn fetch_game_tick_packet_json(window: Window) {
    issue_match_handler_command(&window, &["fetch-gtp".to_owned()], false);
}

#[tauri::command]
pub async fn set_state(window: Window, state: HashMap<String, serde_json::Value>) {
    issue_match_handler_command(&window, &["set_state".to_owned(), serde_json::to_string(&state).unwrap()], false)
}

#[tauri::command]
pub async fn spawn_car_for_viewing(window: Window, config: BotLooksConfig, team: u8, showcase_type: String, map: String) {
    let launcher_settings = LauncherSettings::load(&window);

    let args = [
        "spawn_car_for_viewing".to_owned(),
        serde_json::to_string(&config).unwrap(),
        team.to_string(),
        showcase_type,
        map,
        launcher_settings.preferred_launcher,
        launcher_settings.use_login_tricks.to_string(),
        launcher_settings.rocket_league_exe_path.unwrap_or_default(),
    ];

    issue_match_handler_command(&window, &args, true)
}

#[tauri::command]
pub async fn get_downloaded_botpack_commit_id() -> Option<u32> {
    get_current_tag_name()
}
