use crate::{
    bot_management::{
        bot_creation::{bootstrap_python_bot, bootstrap_python_hivemind, bootstrap_rust_bot, bootstrap_scratch_bot, BoostrapError, CREATED_BOTS_FOLDER},
        downloader::{self, get_current_tag_name, ProgressBarUpdate},
        zip_extract_fixed::{self, ExtractError},
    },
    rlbot::{
        agents::runnable::Runnable,
        gateway_util,
        parsing::{
            agent_config_parser::BotLooksConfig,
            bot_config_bundle::{BotConfigBundle, RLBotCfgParseError, ScriptConfigBundle},
            match_settings_config_parser::{BoostAmount, GameMode, MaxScore, Rumble},
        },
        setup_manager,
    },
    settings::*,
    stories::cmaps::{Bot, BotType, Challenge, City, Script},
    *,
};
use futures_util::StreamExt;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::HashMap,
    error::Error,
    fs::{create_dir_all, File},
    io::{copy, Cursor, Write},
    path::Path,
    time::Instant,
};
use tauri::Window;
use thiserror::Error;
use tokio::{
    fs::File as AsyncFile,
    io::{AsyncReadExt, BufReader},
};

const DEBUG_MODE_SHORT_GAMES: bool = false;
pub const UPDATE_DOWNLOAD_PROGRESS_SIGNAL: &str = "update-download-progress";

#[tauri::command]
pub async fn check_rlbot_python() -> Result<HashMap<String, bool>, String> {
    let mut python_support = HashMap::new();

    let python_path = PYTHON_PATH.read().await.to_owned();

    if get_command_status(&python_path, ["--version"]) {
        python_support.insert("python".to_owned(), true);
        python_support.insert(
            "rlbotpython".to_owned(),
            get_command_status(python_path, ["-c", "import rlbot; import numpy; import numba; import scipy; import selenium"]),
        );
    } else {
        python_support.insert("python".to_owned(), false);
        python_support.insert("rlbotpython".to_owned(), false);
    }

    Ok(dbg!(python_support))
}

fn ensure_bot_directory(window: &Window) -> PathBuf {
    let bot_directory_path = get_content_folder().join(CREATED_BOTS_FOLDER);

    if !bot_directory_path.exists() {
        if let Err(e) = create_dir_all(&bot_directory_path) {
            ccprintln!(window, "Error creating bot directory: {e}");
        }
    }

    bot_directory_path
}

#[derive(Debug, Error)]
pub enum BeginBotError {
    #[error("Failed to create bot template: {0}")]
    Boostraping(#[from] BoostrapError),
    #[error("Failed to load rlbot cfg file: {0}")]
    LoadCfg(#[from] RLBotCfgParseError),
}

#[tauri::command]
pub async fn begin_python_bot(window: Window, bot_name: String) -> Result<BotConfigBundle, String> {
    async fn inner(window: &Window, bot_name: String) -> Result<BotConfigBundle, BeginBotError> {
        let config_file = bootstrap_python_bot(window, bot_name, ensure_bot_directory(window)).await?;
        Ok(BotConfigBundle::minimal_from_path(Path::new(&config_file)).await?)
    }

    inner(&window, bot_name).await.map_err(|e| {
        let err = e.to_string();
        ccprintln(&window, &err);
        err
    })
}

#[tauri::command]
pub async fn begin_python_hivemind(window: Window, hive_name: String) -> Result<BotConfigBundle, String> {
    async fn inner(window: &Window, hive_name: String) -> Result<BotConfigBundle, BeginBotError> {
        let config_file = bootstrap_python_hivemind(window, hive_name, ensure_bot_directory(window)).await?;
        Ok(BotConfigBundle::minimal_from_path(Path::new(&config_file)).await?)
    }

    inner(&window, hive_name).await.map_err(|e| {
        let err = e.to_string();
        ccprintln(&window, &err);
        err
    })
}

#[tauri::command]
pub async fn begin_rust_bot(window: Window, bot_name: String) -> Result<BotConfigBundle, String> {
    async fn inner(window: &Window, bot_name: String) -> Result<BotConfigBundle, BeginBotError> {
        let config_file = bootstrap_rust_bot(window, bot_name, ensure_bot_directory(window)).await?;
        Ok(BotConfigBundle::minimal_from_path(Path::new(&config_file)).await?)
    }

    inner(&window, bot_name).await.map_err(|e| {
        let err = e.to_string();
        ccprintln(&window, &err);
        err
    })
}

#[tauri::command]
pub async fn begin_scratch_bot(window: Window, bot_name: String) -> Result<BotConfigBundle, String> {
    async fn inner(window: &Window, bot_name: String) -> Result<BotConfigBundle, BeginBotError> {
        let config_file = bootstrap_scratch_bot(window, bot_name, ensure_bot_directory(window)).await?;
        Ok(BotConfigBundle::minimal_from_path(Path::new(&config_file)).await?)
    }

    inner(&window, bot_name).await.map_err(|e| {
        let err = e.to_string();
        ccprintln(&window, &err);
        err
    })
}

const PACKAGES: [&str; 9] = [
    "pip",
    "setuptools",
    "wheel",
    "numpy<1.23",
    "scipy",
    "numba<0.56",
    "selenium",
    "rlbot==1.*",
    "rlbot_smh==1.*",
];

/// Apply version constraints to the given package name.
fn get_package_name(package_name: &str) -> &str {
    PACKAGES.into_iter().find(|package| package.contains(package_name)).unwrap_or(package_name)
}

#[tauri::command]
pub async fn install_package(package_string: String) -> Result<PackageResult, String> {
    let exit_code = spawn_capture_process_and_get_exit_code(
        &*PYTHON_PATH.read().await,
        ["-m", "pip", "install", "-U", "--no-warn-script-location", get_package_name(&package_string)],
    );

    Ok(PackageResult::new(exit_code, vec![package_string]))
}

#[derive(Debug, Error)]
pub enum InstallRequirementseError {
    #[error("Failed to load rlbot cfg file: {0}")]
    LoadCfg(#[from] RLBotCfgParseError),
}

#[tauri::command]
pub async fn install_requirements(window: Window, config_path: String) -> Result<PackageResult, String> {
    async fn inner(window: &Window, config_path: String) -> Result<PackageResult, InstallRequirementseError> {
        let bundle = BotConfigBundle::minimal_from_path(Path::new(&config_path)).await?;

        Ok(if let Some(file) = bundle.get_requirements_file() {
            let python = PYTHON_PATH.read().await;
            let packages = bundle.get_missing_packages(window, &*python);
            let exit_code = spawn_capture_process_and_get_exit_code(&*python, ["-m", "pip", "install", "--no-warn-script-location", "-r", file]);

            PackageResult::new(exit_code, packages)
        } else {
            PackageResult::new(1, vec!["unknown file".to_owned()])
        })
    }

    inner(&window, config_path).await.map_err(|e| {
        let err = e.to_string();
        ccprintln(&window, &err);
        err
    })
}

#[tauri::command]
pub async fn install_basic_packages(window: Window) -> Result<PackageResult, String> {
    let packages = PACKAGES.iter().map(ToString::to_string).collect::<Vec<String>>();

    if matches!(online::tokio::check(None).await, Err(_)) {
        ccprintln(
            &window,
            "Error connecting to the internet to install/update basic packages. Please check your internet connection and try again.",
        );

        return Ok(PackageResult::new(3, packages));
    }

    let python = PYTHON_PATH.read().await.to_owned();

    spawn_capture_process_and_get_exit_code(&python, ["-m", "ensurepip"]);

    let mut exit_code = 0;

    for package in PACKAGES {
        exit_code = spawn_capture_process_and_get_exit_code(&python, ["-m", "pip", "install", "-U", "--no-warn-script-location", package]);

        if exit_code != 0 {
            break;
        }
    }

    Ok(PackageResult::new(exit_code, packages))
}

#[tauri::command]
pub fn get_console_texts() -> Result<Vec<String>, String> {
    Ok(CONSOLE_TEXT.lock().map_err(|_| "CONSOLE_TEXT lock was poisoned")?.clone())
}

#[tauri::command]
pub fn get_console_input_commands() -> Result<Vec<String>, String> {
    Ok(CONSOLE_INPUT_COMMANDS.lock().map_err(|_| "CONSOLE_INPUT_COMMANDS lock was poisoned")?.clone())
}

#[tauri::command]
pub async fn run_command(window: Window, input: String) -> Result<(), String> {
    #[cfg(windows)]
    const RLPY: &str = "%rlpy%";
    #[cfg(windows)]
    const RLPY_ESC: &str = "\\%rlpy%";

    #[cfg(not(windows))]
    const RLPY: &str = "$rlpy";
    #[cfg(not(windows))]
    const RLPY_ESC: &str = "\\$rlpy";

    CONSOLE_INPUT_COMMANDS.lock().map_err(|err| err.to_string())?.push(input.clone());

    let python_path_lock = PYTHON_PATH.read().await;
    let (program, original_program) = match input.split_whitespace().next().ok_or_else(|| "No command given".to_string())? {
        RLPY_ESC => (RLPY_ESC, RLPY_ESC),
        RLPY => (python_path_lock.as_ref(), RLPY),
        input => (input, input),
    };

    let args = input.strip_prefix(original_program).and_then(shlex::split).unwrap_or_default();
    spawn_capture_process(program, args).map_err(|err| {
        let e = err.to_string();
        ccprintln(&window, &e);
        e
    })?;

    Ok(())
}

async fn get_missing_packages_generic<T: Runnable + Send + Sync>(window: &Window, runnables: Vec<T>) -> Vec<MissingPackagesUpdate> {
    if check_has_rlbot().await {
        let python_path = PYTHON_PATH.read().await.to_owned();
        runnables
            .par_iter()
            .enumerate()
            .filter_map(|(index, runnable)| {
                if runnable.is_rlbot_controlled() && runnable.may_require_python_packages() {
                    let mut warn = runnable.warn().clone();
                    let mut missing_packages = runnable.missing_python_packages().clone();

                    if let Some(missing_packages) = &missing_packages {
                        if warn == Some("pythonpkg".to_owned()) && missing_packages.is_empty() {
                            warn = None;
                        }
                    } else {
                        let bot_missing_packages = runnable.get_missing_packages(window, &python_path);

                        if bot_missing_packages.is_empty() {
                            warn = None;
                        } else {
                            warn = Some("pythonpkg".to_owned());
                        }

                        missing_packages = Some(bot_missing_packages);
                    }

                    if &warn != runnable.warn() || &missing_packages != runnable.missing_python_packages() {
                        return Some(MissingPackagesUpdate { index, warn, missing_packages });
                    }
                }

                None
            })
            .collect()
    } else {
        runnables
            .par_iter()
            .enumerate()
            .filter_map(|(index, runnable)| {
                if runnable.is_rlbot_controlled() && (runnable.warn().is_some() || runnable.missing_python_packages().is_some()) {
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
pub async fn get_missing_bot_packages(window: Window, bots: Vec<BotConfigBundle>) -> Vec<MissingPackagesUpdate> {
    get_missing_packages_generic(&window, bots).await
}

#[tauri::command]
pub async fn get_missing_script_packages(window: Window, scripts: Vec<ScriptConfigBundle>) -> Vec<MissingPackagesUpdate> {
    get_missing_packages_generic(&window, scripts).await
}

fn get_missing_logos_generic<T: Runnable + Send + Sync>(runnables: &[T]) -> Vec<LogoUpdate> {
    runnables
        .par_iter()
        .enumerate()
        .filter_map(|(index, runnable)| {
            if runnable.is_rlbot_controlled() && runnable.logo().is_none() {
                if let Some(logo) = runnable.load_logo() {
                    return Some(LogoUpdate { index, logo });
                }
            }

            None
        })
        .collect()
}

#[tauri::command]
pub fn get_missing_bot_logos(bots: Vec<BotConfigBundle>) -> Vec<LogoUpdate> {
    get_missing_logos_generic(&bots)
}

#[tauri::command]
pub fn get_missing_script_logos(scripts: Vec<ScriptConfigBundle>) -> Vec<LogoUpdate> {
    get_missing_logos_generic(&scripts)
}

#[tauri::command]
pub fn is_windows() -> bool {
    cfg!(windows)
}

#[derive(Debug, Error)]
pub enum BootstrapCustomPythonError {
    #[error("This function is only supported on Windows")]
    NotWindows,
    #[error("Couldn't download the custom python zip: {0}")]
    Download(#[from] reqwest::Error),
    #[error(transparent)]
    EmitSignal(#[from] tauri::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Coudn't extract the zip: {0}")]
    ExtractZip(#[from] ExtractError),
}

/// Downloads RLBot's isloated Python 3.7.9 environment and unzips it.
/// Updates the user with continuous progress updates.
///
/// WORKS FOR WINDOWS ONLY
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
pub async fn bootstrap_custom_python(window: &Window) -> Result<(), BootstrapCustomPythonError> {
    if cfg!(not(windows)) {
        return Err(BootstrapCustomPythonError::NotWindows);
    }

    let content_folder = get_content_folder();
    let folder_destination = content_folder.join("Python37");
    let file_path = content_folder.join("python-3.7.9-custom-amd64.zip");

    let download_url = "https://virxec.github.io/rlbot_gui_rust/python-3.7.9-custom-amd64.zip";
    let res = reqwest::Client::new().get(download_url).send().await?;
    let total_size = 21_873_000;
    let mut stream = res.bytes_stream();
    let mut bytes = Vec::with_capacity(total_size);
    let mut last_update = Instant::now();

    if !file_path.exists() {
        while let Some(new_bytes) = stream.next().await {
            // put the new bytes into bytes
            bytes.extend_from_slice(&new_bytes?);

            if last_update.elapsed().as_secs_f32() >= 0.1 {
                let progress = bytes.len() as f32 / total_size as f32 * 100.0;
                window.emit(UPDATE_DOWNLOAD_PROGRESS_SIGNAL, ProgressBarUpdate::new(progress, "Downloading zip...".to_owned()))?;
                last_update = Instant::now();
            }
        }

        window.emit(UPDATE_DOWNLOAD_PROGRESS_SIGNAL, ProgressBarUpdate::new(100., "Writing zip to disk...".to_owned()))?;

        let mut file = File::create(&file_path)?;
        let mut content = Cursor::new(bytes);
        copy(&mut content, &mut file)?;
    }

    window.emit(UPDATE_DOWNLOAD_PROGRESS_SIGNAL, ProgressBarUpdate::new(100., "Extracting zip...".to_owned()))?;

    // Extract the zip file
    zip_extract_fixed::extract(window, File::open(&file_path)?, folder_destination.as_path(), false, false)?;

    // Update the Python path
    *PYTHON_PATH.write().await = folder_destination.join("python.exe").to_string_lossy().to_string();

    Ok(())
}

#[tauri::command]
pub async fn install_python(window: Window) -> Result<(), String> {
    bootstrap_custom_python(&window).await.map_err(|e| {
        let e = e.to_string();
        ccprintln(&window, &e);
        e
    })
}

#[derive(Debug, Error)]
pub enum VenvCreationError {
    #[error("Failed to create virtual environment at {0}")]
    Creation(String),
    #[error("Python was not properly installed ({0})")]
    ImproperInstallation(String),
}

#[tauri::command]
pub async fn create_python_venv(window: Window, path: String) -> Result<(), String> {
    async fn inner(path: String) -> Result<(), VenvCreationError> {
        let python_folder = get_content_folder().join("env");
        let python_folder_str = python_folder.to_string_lossy().to_string();
        if !get_command_status(path, ["-m", "venv", &python_folder_str]) {
            return Err(VenvCreationError::Creation(python_folder_str));
        }

        let python_path = python_folder.join("bin/python").to_string_lossy().to_string();
        if !get_command_status(&python_path, ["--version"]) {
            return Err(VenvCreationError::ImproperInstallation(python_path));
        }

        *PYTHON_PATH.write().await = python_path;

        Ok(())
    }

    inner(path).await.map_err(|e| {
        let e = e.to_string();
        ccprintln(&window, &e);
        e
    })
}

#[tauri::command]
pub async fn download_bot_pack(window: Window) -> Result<String, String> {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_string_lossy().to_string();
    let botpack_status = downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await;

    Ok(match botpack_status {
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.write().await.add_folder(&window, botpack_location);
            message
        }
        downloader::BotpackStatus::Skipped(message) => message,
        downloader::BotpackStatus::RequiresFullDownload => unreachable!(),
    })
}

#[tauri::command]
pub async fn update_bot_pack(window: Window) -> Result<String, String> {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_string_lossy().to_string();
    let botpack_status = downloader::update_bot_pack(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location).await;

    Ok(match botpack_status {
        downloader::BotpackStatus::Skipped(message) => message,
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.write().await.add_folder(&window, botpack_location);
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.write().await.add_folder(&window, botpack_location);
                    message
                }
                downloader::BotpackStatus::Skipped(message) => message,
                downloader::BotpackStatus::RequiresFullDownload => unreachable!(),
            }
        }
    })
}

#[tauri::command]
pub async fn update_map_pack(window: Window) -> Result<String, String> {
    let mappack_location = get_content_folder().join(MAPPACK_FOLDER);
    let updater = downloader::MapPackUpdater::new(&mappack_location, MAPPACK_REPO.0.to_owned(), MAPPACK_REPO.1.to_owned());
    let location = mappack_location.to_string_lossy().to_string();

    Ok(match updater.needs_update(&window).await {
        downloader::BotpackStatus::Skipped(message) | downloader::BotpackStatus::Success(message) => {
            BOT_FOLDER_SETTINGS.write().await.add_folder(&window, location);
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, MAPPACK_REPO.0, MAPPACK_REPO.1, &location, false).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.write().await.add_folder(&window, location);

                    if updater.get_map_index(&window).await.is_none() {
                        ccprintln(&window, "Error: Couldn't find revision number in map pack");
                        return Err("Couldn't find revision number in map pack".to_owned());
                    }

                    let map_index_old = updater.get_map_index(&window).await;
                    updater.hydrate_map_pack(&window, map_index_old).await;

                    message
                }
                downloader::BotpackStatus::Skipped(message) => message,
                downloader::BotpackStatus::RequiresFullDownload => unreachable!(),
            }
        }
    })
}

#[tauri::command]
pub async fn is_botpack_up_to_date(window: Window) -> bool {
    let repo_full_name = format!("{BOTPACK_REPO_OWNER}/{BOTPACK_REPO_NAME}");
    bot_management::downloader::is_botpack_up_to_date(&window, &repo_full_name).await
}

#[tauri::command]
pub async fn get_launcher_settings(window: Window) -> LauncherConfig {
    LauncherConfig::load(&window).await
}

#[tauri::command]
pub async fn save_launcher_settings(window: Window, settings: LauncherConfig) {
    settings.write_to_file(&window).await;
}

/// Starts the match handler, which is written in Python so it can use the RLBot package (also written in Python)
///
/// Returns None if it fails, otherwise returns pipe for the child process's stdin
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
fn create_match_handler<S: AsRef<OsStr>>(window: &Window, use_pipe: bool, python_path: S) -> Option<(String, ChildStdin)> {
    match get_maybe_capture_command(&python_path, ["-c", "from rlbot_smh.match_handler import listen; listen()"], use_pipe)
        .ok()?
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => child.stdin.take().map(|stdin| (python_path.as_ref().to_string_lossy().to_string(), stdin)),
        Err(err) => {
            ccprintln!(window, "Error starting match handler: {err}");
            None
        }
    }
}

enum CreateHandler {
    /// The bool is whether is not a pipe should be attached to the process
    Yes(bool),
    No,
}

/// Send a command to the match handler
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `command` - The command to send to the match handler - can be in multiple parts, for passing arguments
/// * `create_handler` - If the match handler should be started if it's down
fn issue_match_handler_command<S: AsRef<OsStr>>(window: &Window, command_parts: &[String], mut create_handler: CreateHandler, python_path: S) -> Result<(), String> {
    let mut command_lock = MATCH_HANDLER_STDIN.lock().map_err(|err| err.to_string())?;
    let (used_py_path, match_handler_stdin) = &mut *command_lock;

    if match_handler_stdin.is_none() {
        if let CreateHandler::Yes(use_pipe) = create_handler {
            ccprintln(window, "Starting match handler!");
            if let Some((py_path, stdin)) = create_match_handler(window, use_pipe, &python_path) {
                *match_handler_stdin = Some(stdin);
                *used_py_path = py_path;
                create_handler = CreateHandler::No;
            } else {
                return Err("Couldn't start match handler".to_owned());
            }
        } else {
            ccprintln(window, "Not issuing command to handler as it's down and I was told to not start it");
            return Ok(());
        }
    }

    let command = format!("{} | \n", command_parts.join(" | "));
    let stdin = match_handler_stdin.as_mut().ok_or("Tried creating match handler but failed")?;

    if stdin.write_all(command.as_bytes()).is_err() {
        drop(match_handler_stdin.take());
        if matches!(create_handler, CreateHandler::Yes(_)) {
            ccprintln(window, "Failed to write to match handler, trying to restart...");
            issue_match_handler_command(window, command_parts, create_handler, python_path)
        } else {
            Err("Failed to write to match handler".to_owned())
        }
    } else {
        Ok(())
    }
}

/// Perform pre-match startup checks
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
async fn pre_start_match(window: &Window) -> Result<(), String> {
    let port = gateway_util::find_existing_process(window);
    let rl_is_running = setup_manager::is_rocket_league_running(port.unwrap_or(gateway_util::IDEAL_RLBOT_PORT))?;

    ccprintln(
        window,
        format!("Rocket League is {}", if rl_is_running { "already running with RLBot args!" } else { "not running yet..." }),
    );

    if port.is_some() {
        // kill the current bots if they're running
        kill_bots(window.clone()).await?;

        // kill RLBot if it's running but Rocket League isn't
        if !rl_is_running {
            gateway_util::kill_existing_processes(window);
        }
    }

    Ok(())
}

/// Starts a match via the match handler with the given settings
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `bot_list` - A list of bots and their settings to use in the match
/// * `match_settings` - The various match settings to use in the match, including scripts (only the path), mutators, game map, etc.
async fn start_match_helper(window: &Window, bot_list: Vec<TeamBotBundle>, match_settings: MiniMatchConfig, use_pipe: bool) -> Result<(), String> {
    pre_start_match(window).await?;

    let launcher_settings = LauncherConfig::load(window).await;
    let match_settings = match_settings.setup_for_start_match(window, &BOT_FOLDER_SETTINGS.read().await.folders)?;

    let args = [
        "start_match".to_owned(),
        serde_json::to_string(&bot_list).map_err(|e| e.to_string())?.as_str().to_owned(),
        serde_json::to_string(&match_settings).map_err(|e| e.to_string())?.as_str().to_owned(),
        launcher_settings.preferred_launcher,
        launcher_settings.use_login_tricks.to_string(),
        launcher_settings.rocket_league_exe_path.unwrap_or_default(),
    ];

    println!("Issuing command: {} | ", args.join(" | "));

    issue_match_handler_command(window, &args, CreateHandler::Yes(use_pipe), &*PYTHON_PATH.read().await)?;

    Ok(())
}

#[tauri::command]
pub async fn start_match(window: Window, bot_list: Vec<TeamBotBundle>, match_settings: MiniMatchConfig) -> Result<(), String> {
    start_match_helper(&window, bot_list, match_settings, USE_PIPE.load(Ordering::Relaxed))
        .await
        .map_err(|error| {
            if let Err(e) = window.emit("match-start-failed", ()) {
                ccprintln!(&window, "Failed to emit match-start-failed: {e}");
            }

            ccprintln(&window, &error);

            error
        })
}

#[tauri::command]
pub async fn kill_bots(window: Window) -> Result<(), String> {
    issue_match_handler_command(&window, &["kill_bots".to_owned()], CreateHandler::No, "")?;

    Ok(())
}

#[tauri::command]
pub async fn shut_down_match_handler() -> Result<(), String> {
    let mut stdin_lock = MATCH_HANDLER_STDIN.lock().map_err(|err| err.to_string())?;
    if let (_, Some(stdin)) = &mut *stdin_lock {
        const KILL_BOTS_COMMAND: &[u8] = "shut_down | \n".as_bytes();
        stdin.write_all(KILL_BOTS_COMMAND).map_err(|err| err.to_string())?;
    }

    stdin_lock.1 = None;

    Ok(())
}

#[tauri::command]
pub async fn fetch_game_tick_packet_json(window: Window) -> Result<(), String> {
    issue_match_handler_command(&window, &["fetch_gtp".to_owned()], CreateHandler::No, &*PYTHON_PATH.read().await)?;
    Ok(())
}

#[tauri::command]
pub async fn set_state(window: Window, state: HashMap<String, serde_json::Value>) -> Result<(), String> {
    issue_match_handler_command(
        &window,
        &["set_state".to_owned(), serde_json::to_string(&state).map_err(|e| e.to_string())?],
        CreateHandler::No,
        &*PYTHON_PATH.read().await,
    )
}

#[tauri::command]
pub async fn spawn_car_for_viewing(window: Window, config: BotLooksConfig, team: u8, showcase_type: String, map: String) -> Result<(), String> {
    let launcher_settings = LauncherConfig::load(&window).await;

    let args = [
        "spawn_car_for_viewing".to_owned(),
        serde_json::to_string(&config).map_err(|e| e.to_string())?,
        team.to_string(),
        showcase_type,
        map,
        launcher_settings.preferred_launcher,
        launcher_settings.use_login_tricks.to_string(),
        launcher_settings.rocket_league_exe_path.unwrap_or_default(),
    ];

    issue_match_handler_command(&window, &args, CreateHandler::Yes(true), &*PYTHON_PATH.read().await)
}

#[tauri::command]
pub async fn get_downloaded_botpack_commit_id() -> Option<u32> {
    get_current_tag_name().await
}

/// Creates a `TeamBotBundle` that represents the human player
///
/// # Arguments
///
/// * `team` - The team the human player should be on
fn make_human_config(team: Team) -> TeamBotBundle {
    TeamBotBundle {
        name: "Human".to_owned(),
        team,
        skill: 1.0,
        runnable_type: "human".to_owned(),
        path: None,
    }
}

/// Collapses a path, e.x. `["$RLBOTPACKROOT", "RLBotPack", "Kamael_family", "Kamael.cfg"]`, to the actual path on the file system
///
/// # Arguments
///
/// * `path` - The un-parsed JSON path to collapse
/// * `botpack_root` - The path to the root of the RLBotPack, which will replace `$RLBOTPACKROOT`
fn collapse_path(cfg_path: Option<&Vec<String>>, botpack_root: &Path) -> Option<String> {
    let cfg_path = cfg_path?;

    let mut path = PathBuf::new();

    for part in cfg_path {
        if part == "$RLBOTPACKROOT" {
            path.push(botpack_root);
        } else {
            path.push(part);
        }
    }

    Some(path.to_string_lossy().to_string())
}

/// Load a RLBot-type bot
///
/// # Arguments
///
/// `player` - The JSON map that contains the bot's config
/// `team` - The team the bot should be on
/// `botpack_root` - The path to the root of the RLBotPack, which will replace `$RLBOTPACKROOT`
fn rlbot_to_player_config(player: &Bot, team: Team, botpack_root: &Path) -> TeamBotBundle {
    TeamBotBundle {
        name: player.name.clone(),
        team,
        skill: 1.0,
        runnable_type: "rlbot".to_owned(),
        path: Some(collapse_path(player.path.as_ref(), botpack_root).unwrap_or_default()),
    }
}

/// Load a psyonix-type bot
///
/// # Arguments
///
/// `player` - The JSON map that contains the bot's config
/// `team` - The team the bot should be on
fn pysonix_to_player_config(player: &Bot, team: Team) -> TeamBotBundle {
    TeamBotBundle {
        name: player.name.clone(),
        team,
        skill: player.skill.unwrap_or(1.0) as f32,
        runnable_type: "psyonix".to_owned(),
        path: None,
    }
}

/// Load a `TeamBotBundle` from a Bot
///
/// # Arguments
///
/// `player` - The JSON map that contains the bot's config
/// `team` - The team the bot should be on
/// `botpack_root` - The path to the root of the RLBotPack, which will replace `$RLBOTPACKROOT`
fn bot_to_team_bot_bundle(player: &Bot, team: Team, botpack_root: &Path) -> TeamBotBundle {
    if player.type_field == BotType::Psyonix {
        pysonix_to_player_config(player, team)
    } else {
        rlbot_to_player_config(player, team, botpack_root)
    }
}

/// Load all the bots (+ the human) for a challenge
///
/// # Arguments
///
/// * `challenge` - The JSON map that contains the key `humanTeamSize`
/// * `human_pick` - The names of the bots that the human picked for teammates
/// * `all_bots` - The JSON that contains a mapping of bot names to bot information
/// * `botpack_root` - The path to the root of the RLBotPack, which will replace `$RLBOTPACKROOT`
fn make_player_configs(challenge: &Challenge, human_picks: &[String], all_bots: &HashMap<String, Bot>, botpack_root: &Path) -> Vec<TeamBotBundle> {
    let mut player_configs = vec![make_human_config(Team::Blue)];

    for name in human_picks[..challenge.human_team_size as usize - 1].iter() {
        if let Some(bot) = all_bots.get(name) {
            player_configs.push(bot_to_team_bot_bundle(bot, Team::Blue, botpack_root));
        }
    }

    for opponent in &challenge.opponent_bots {
        if let Some(bot) = all_bots.get(opponent) {
            player_configs.push(bot_to_team_bot_bundle(bot, Team::Orange, botpack_root));
        }
    }

    player_configs
}

/// Load a script from a Script config
///
/// # Arguments
///
/// * `script` - The JSON map that the key "path" which points to the script's .py file
/// * `botpack_root` - The path to the root of the RLBotPack, which will replace `$RLBOTPACKROOT`
fn script_to_miniscript_bundle(script: &Script, botpack_root: &Path) -> MiniScriptBundle {
    MiniScriptBundle {
        path: collapse_path(Some(&script.path), botpack_root).unwrap_or_default(),
    }
}

/// Load all of the scripts for a challenge
///
/// # Arguments
///
/// * `challenge` - The JSON map that contains the key `scripts`
/// * `all_scripts` - The JSON that contains a mapping of script names to script information
/// * `botpack_root` - The path to the root of the RLBotPack, which will replace `$RLBOTPACKROOT`
fn make_script_configs(challenge: &Challenge, all_scripts: &HashMap<String, Script>, botpack_root: &Path) -> Vec<MiniScriptBundle> {
    challenge
        .scripts
        .iter()
        .filter_map(|script| all_scripts.get(script))
        .map(|script| script_to_miniscript_bundle(script, botpack_root))
        .collect()
}

/// Load the match settings for a challenge
///
/// # Arguments
///
/// * `challenge` - The JSON map that contains the key `matchSettings`
/// * `upgrades` - The purchased upgrades
/// * `script_configs` - The loaded scripts that will be used in the challenge
fn make_match_config(challenge: &Challenge, upgrades: &HashMap<String, usize>, script_configs: Vec<MiniScriptBundle>) -> MiniMatchConfig {
    MiniMatchConfig {
        game_mode: challenge.limitations.contains(&"half-field".to_owned()) // check if the vec contains the string "half-field"
            .then_some(GameMode::Heatseeker) // if it does, set the game mode to Heatseeker
            .unwrap_or_default(), // otherwise, set it to Soccer
        map: challenge.map.clone(),
        enable_state_setting: true,
        scripts: script_configs,
        mutators: MutatorConfig {
            max_score: if DEBUG_MODE_SHORT_GAMES {
                MaxScore::ThreeGoals
            } else {
                // config-defined or unlimited
                challenge.max_score
            },
            boost_amount: challenge.disabled_boost.then_some(BoostAmount::NoBoost).unwrap_or_default(), // config-defined or normal
            rumble: upgrades.contains_key("rumble").then_some(Rumble::Default).unwrap_or_default(),     // Rumble default / none
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Find the challenge with the given ID in the given city
///
/// # Arguments
///
/// * `challenge_id` - The ID of the challenge to find
/// * `city` - The city to search in
fn find_challenge_in_city(challenge_id: &str, city: &City) -> Option<Challenge> {
    city.challenges.iter().find(|x| x.id == challenge_id).cloned()
}

/// Find the challenge and associated city from the given challenge ID
///
/// # Arguments
///
/// * `story_settings` - Information on the story configuration, used to load the inforamation about the cities and challenges
/// * `challenge_id` - The ID of the challenge to find
async fn get_challenge_by_id(story_settings: &StoryConfig, challenge_id: &str) -> Option<(City, Challenge)> {
    get_cities(story_settings)
        .await
        .values()
        .find_map(|city| find_challenge_in_city(challenge_id, city).map(|challenge| (city.clone(), challenge)))
}

/// Launch a challenge for the user to play
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `story_save` - The save state of the story, containing all the information about the story
/// * `challenge_id` - The ID of the challenge to run
/// * `picked_teammates` - The teammates that were picked by the human for teammates to use in the challenge
async fn run_challenge(window: &Window, save_state: &StoryState, challenge_id: String, picked_teammates: &[String]) -> Result<(), Box<dyn Error>> {
    pre_start_match(window).await?;

    let story_settings = save_state.get_story_settings();

    let (city, challenge) = match get_challenge_by_id(story_settings, &challenge_id).await {
        Some(challenge) => challenge,
        None => return Err(format!("Could not find challenge with id {challenge_id}").into()),
    };

    let all_bots = get_all_bot_configs(story_settings).await;
    let all_scripts = get_all_script_configs(story_settings).await;

    let botpack_root = match BOT_FOLDER_SETTINGS
        .read()
        .await
        .folders
        .keys()
        .map(|bf| Path::new(bf).join("RLBotPack-master"))
        .find(|bf| bf.exists())
    {
        Some(bf) => bf,
        None => return Err("Could not find RLBotPack-master folder".into()),
    };

    let player_configs = make_player_configs(&challenge, picked_teammates, &all_bots, botpack_root.as_path());
    let match_settings = make_match_config(&challenge, save_state.get_upgrades(), make_script_configs(&challenge, &all_scripts, botpack_root.as_path()));
    let launcher_prefs = LauncherConfig::load(window).await;

    let args = [
        "launch_challenge".to_owned(),
        challenge_id,
        serde_json::to_string(&city.description.color).map_err(|e| e.to_string())?,
        serde_json::to_string(&save_state.get_team_settings().color).map_err(|e| e.to_string())?,
        serde_json::to_string(&save_state.get_upgrades()).map_err(|e| e.to_string())?,
        serde_json::to_string(&player_configs).map_err(|e| e.to_string())?,
        serde_json::to_string(&match_settings).map_err(|e| e.to_string())?,
        serde_json::to_string(&challenge).map_err(|e| e.to_string())?,
        serde_json::to_string(&save_state).map_err(|e| e.to_string())?,
        launcher_prefs.preferred_launcher,
        launcher_prefs.use_login_tricks.to_string(),
        launcher_prefs.rocket_league_exe_path.unwrap_or_default(),
    ];

    println!("Issuing command: {} | ", args.join(" | "));

    issue_match_handler_command(window, &args, CreateHandler::Yes(true), &*PYTHON_PATH.read().await)?;

    Ok(())
}

#[tauri::command]
pub async fn launch_challenge(window: Window, save_state: StoryState, challenge_id: String, picked_teammates: Vec<String>) -> Result<(), String> {
    run_challenge(&window, &save_state, challenge_id, &picked_teammates).await.map_err(|err| {
        if let Err(e) = window.emit("match-start-failed", ()) {
            ccprintln!(&window, "Failed to emit match-start-failed: {e}");
        }

        let e = err.to_string();
        ccprintln(&window, &e);
        e
    })
}

#[tauri::command]
pub async fn purchase_upgrade(window: Window, mut save_state: StoryState, upgrade_id: String, cost: usize) -> Option<StoryState> {
    if let Err(e) = save_state.add_purchase(upgrade_id, cost) {
        ccprintln(&window, e);
        return None;
    }

    save_state.save(&window).await;

    Some(save_state)
}

#[tauri::command]
pub async fn recruit(window: Window, mut save_state: StoryState, id: String) -> Option<StoryState> {
    if let Err(e) = save_state.add_recruit(id) {
        ccprintln(&window, e);
        return None;
    }

    save_state.save(&window).await;

    Some(save_state)
}

#[derive(Debug, Error)]
pub enum LogUploadError {
    #[error("Failed to read log file: {0}")]
    IO(#[from] std::io::Error),
    #[error("The log file is empty; not uploading")]
    EmptyLog,
    #[error("Failed to upload log file: {0}")]
    Upload(#[from] reqwest::Error),
    #[error("No key '{0}' in JSON repsonse")]
    NoKey(String),
    #[error("Key '{0}' in JSON response was not a string")]
    InvalidKeyType(String),
}

#[tauri::command]
pub async fn upload_log(window: Window) -> Result<String, String> {
    const KEY: &str = "key";

    /// there are a few references to hastebin in the GUI
    /// you should also change those references to avoid user confusion
    /// (if changing paste provider)
    async fn inner(window: &Window) -> Result<String, LogUploadError> {
        ccprintln(window, "Reading log file...");
        let file = AsyncFile::open(get_log_path()).await?;

        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        reader.read_to_string(&mut contents).await?;

        if contents.is_empty() {
            return Err(LogUploadError::EmptyLog);
        }

        let (home_folder, replacement_key) = get_home_folder();
        let contents = contents.replace(&home_folder.to_string_lossy().to_string(), replacement_key);

        ccprintln(window, "Log file read succesfully! Now uploading to hastebin...");

        // send contents to https://hastebin.com/documents via POST
        let res = reqwest::Client::new().post("https://hastebin.com/documents").body(contents).send().await?;

        // the returned JSON looks a little bit like `{"key":"royidegeni"}`
        // Take this and return the key attached to the hastebin URL
        let json: serde_json::Value = res.json().await?;

        let url_postfix = json
            .get(KEY)
            .ok_or_else(|| LogUploadError::NoKey(KEY.to_owned()))?
            .as_str()
            .ok_or_else(|| LogUploadError::InvalidKeyType(KEY.to_owned()))?
            .to_owned();

        let url = format!("https://hastebin.com/{url_postfix}");
        ccprintln!(window, "Log file uploaded to: {url}");
        Ok(url)
    }

    inner(&window).await.map_err(|e| {
        let err = e.to_string();
        ccprintln(&window, &err);
        err
    })
}
