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
    stories::{Bot, BotType, Challenge, City, Script},
    *,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use flate2::{write::GzEncoder, Compression};
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
use thiserror::Error;
use tokio::{
    fs::File as AsyncFile,
    io::{AsyncReadExt, BufReader},
};

const DEBUG_MODE_SHORT_GAMES: bool = false;
pub const UPDATE_DOWNLOAD_PROGRESS_SIGNAL: &str = "update-download-progress";

#[tauri::command]
pub async fn check_rlbot_python() -> HashMap<String, bool> {
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

    dbg!(python_support)
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

impl_serialize_from_display!(BeginBotError);

#[tauri::command]
pub async fn begin_python_bot(window: Window, bot_name: String) -> Result<BotConfigBundle, BeginBotError> {
    let config_file = bootstrap_python_bot(&window, bot_name, ensure_bot_directory(&window)).await?;
    Ok(BotConfigBundle::minimal_from_path(config_file).await?)
}

#[tauri::command]
pub async fn begin_python_hivemind(window: Window, hive_name: String) -> Result<BotConfigBundle, BeginBotError> {
    let config_file = bootstrap_python_hivemind(&window, hive_name, ensure_bot_directory(&window)).await?;
    Ok(BotConfigBundle::minimal_from_path(config_file).await?)
}

#[tauri::command]
pub async fn begin_rust_bot(window: Window, bot_name: String) -> Result<BotConfigBundle, BeginBotError> {
    let config_file = bootstrap_rust_bot(&window, bot_name, ensure_bot_directory(&window)).await?;
    Ok(BotConfigBundle::minimal_from_path(config_file).await?)
}

#[tauri::command]
pub async fn begin_scratch_bot(window: Window, bot_name: String) -> Result<BotConfigBundle, BeginBotError> {
    let config_file = bootstrap_scratch_bot(&window, bot_name, ensure_bot_directory(&window)).await?;
    Ok(BotConfigBundle::minimal_from_path(config_file).await?)
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
    "rlbot_smh>=1.0.13",
];

/// Apply version constraints to the given package name.
fn get_package_name(package_name: &str) -> &str {
    PACKAGES.into_iter().find(|package| package.contains(package_name)).unwrap_or(package_name)
}

#[tauri::command]
pub async fn install_package(package_string: String) -> PackageResult {
    let exit_code = spawn_capture_process_and_get_exit_code(
        &*PYTHON_PATH.read().await,
        ["-m", "pip", "install", "-U", "--no-warn-script-location", get_package_name(&package_string)],
    );

    PackageResult::new(exit_code, vec![package_string])
}

#[derive(Debug, Error)]
pub enum InstallRequirementseError {
    #[error("Failed to load rlbot cfg file: {0}")]
    LoadCfg(#[from] RLBotCfgParseError),
}

impl_serialize_from_display!(InstallRequirementseError);

#[tauri::command]
pub async fn install_requirements(window: Window, config_path: String) -> Result<PackageResult, InstallRequirementseError> {
    let bundle = BotConfigBundle::minimal_from_path(Path::new(&config_path)).await?;

    Ok(if let Some(file) = bundle.get_requirements_file() {
        let python = PYTHON_PATH.read().await;
        let packages = bundle.get_missing_packages(&window, &*python);
        let exit_code = spawn_capture_process_and_get_exit_code(&*python, ["-m", "pip", "install", "--no-warn-script-location", "-r", file]);

        PackageResult::new(exit_code, packages)
    } else {
        PackageResult::new(1, vec!["unknown file".to_owned()])
    })
}

#[tauri::command]
pub async fn install_basic_packages(window: Window) -> PackageResult {
    let packages = PACKAGES.iter().map(ToString::to_string).collect::<Vec<String>>();

    if matches!(online::tokio::check(None).await, Err(_)) {
        ccprintln(
            &window,
            "Error connecting to the internet to install/update basic packages. Please check your internet connection and try again.",
        );

        return PackageResult::new(3, packages);
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

    PackageResult::new(exit_code, packages)
}

#[tauri::command]
pub fn get_console_texts() -> Result<Vec<String>, String> {
    Ok(CONSOLE_TEXT.lock().map_err(|_| "Mutex CONSOLE_TEXT was poisoned")?.clone())
}

#[tauri::command]
pub fn get_console_input_commands() -> Result<Vec<String>, String> {
    Ok(CONSOLE_INPUT_COMMANDS.lock().map_err(|_| "Mutex CONSOLE_INPUT_COMMANDS was poisoned")?.clone())
}

#[derive(Debug, Error)]
pub enum RunCommandError {
    #[error("No command given")]
    NoCommand,
    #[error("Mutex {0} was poisoned")]
    Poisoned(&'static str),
    #[error(transparent)]
    Command(#[from] CommandError),
}

impl_serialize_from_display!(RunCommandError);

#[tauri::command]
pub async fn run_command(window: Window, input: String) -> Result<(), RunCommandError> {
    #[cfg(windows)]
    const RLPY: &str = "%rlpy%";
    #[cfg(windows)]
    const RLPY_ESC: &str = "\\%rlpy%";

    #[cfg(not(windows))]
    const RLPY: &str = "$rlpy";
    #[cfg(not(windows))]
    const RLPY_ESC: &str = "\\$rlpy";

    CONSOLE_INPUT_COMMANDS
        .lock()
        .map_err(|_| RunCommandError::Poisoned("CONSOLE_INPUT_COMMANDS"))?
        .push(input.clone());

    let python_path_lock = PYTHON_PATH.read().await;
    let (program, original_program) = match input.split_whitespace().next().ok_or(RunCommandError::NoCommand)? {
        RLPY_ESC => (RLPY_ESC, RLPY_ESC),
        RLPY => (python_path_lock.as_ref(), RLPY),
        input => (input, input),
    };

    let args = input.strip_prefix(original_program).and_then(shlex::split).unwrap_or_default();
    spawn_capture_process(program, args).map_err(|err| {
        ccprintln(&window, err.to_string());
        err
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
                    let mut warn = runnable.warn().as_deref();
                    let missing_packages = Some(if let Some(missing_packages) = runnable.missing_python_packages() {
                        if warn == Some("pythonpkg") && missing_packages.is_empty() {
                            warn = None;
                        }

                        missing_packages.clone()
                    } else {
                        let bot_missing_packages = runnable.get_missing_packages(window, &python_path);

                        if bot_missing_packages.is_empty() {
                            warn = None;
                        } else {
                            warn = Some("pythonpkg");
                        }

                        bot_missing_packages
                    });

                    if warn != runnable.warn().as_deref() || &missing_packages != runnable.missing_python_packages() {
                        return Some(MissingPackagesUpdate {
                            index,
                            warn: warn.map(String::from),
                            missing_packages,
                        });
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
                    Some(MissingPackagesUpdate { index, ..Default::default() })
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

impl_serialize_from_display!(BootstrapCustomPythonError);

/// Downloads `RLBot`'s isloated Python 3.7.9 environment and unzips it.
/// Updates the user with continuous progress updates.
///
/// WORKS FOR WINDOWS ONLY
#[tauri::command]
pub async fn install_python(window: Window) -> Result<(), BootstrapCustomPythonError> {
    if cfg!(not(windows)) {
        return Err(BootstrapCustomPythonError::NotWindows);
    }

    let content_folder = get_content_folder();
    let folder_destination = content_folder.join("Python37");
    let file_path = content_folder.join("python-3.7.9-custom-amd64.zip");

    let download_url = "https://virxec.github.io/rlbot_gui_rust/python-3.7.9-custom-amd64.zip";
    let res = reqwest::Client::new().get(download_url).send().await?;
    let total_size: u32 = 21_873_000;
    let mut stream = res.bytes_stream();
    let mut bytes = Vec::with_capacity(total_size as usize);
    let mut last_update = Instant::now();
    let total_size = f64::from(total_size);

    if !file_path.exists() {
        while let Some(new_bytes) = stream.next().await {
            // put the new bytes into bytes
            bytes.extend_from_slice(&new_bytes?);

            if last_update.elapsed().as_secs_f32() >= 0.1 {
                let progress = bytes.len() as f64 / total_size * 100.0;
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
    zip_extract_fixed::extract(&window, File::open(&file_path)?, folder_destination.as_path(), false, false)?;

    // Update the Python path
    *PYTHON_PATH.write().await = folder_destination.join("python.exe").to_string_lossy().to_string();

    Ok(())
}

#[derive(Debug, Error)]
pub enum VenvCreationError {
    #[error("Failed to create virtual environment at {0}")]
    Creation(String),
    #[error("Python was not properly installed ({0})")]
    ImproperInstallation(String),
}

impl_serialize_from_display!(VenvCreationError);

#[tauri::command]
pub async fn create_python_venv(path: String) -> Result<(), VenvCreationError> {
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

#[derive(Debug, Error)]
pub enum BotPackError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Couldn't find revision number in map pack")]
    NoRevision,
}

impl_serialize_from_display!(BotPackError);

#[tauri::command]
pub async fn download_bot_pack(window: Window) -> Result<String, BotPackError> {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_string_lossy().to_string();
    let botpack_status = downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await;

    Ok(match botpack_status {
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.write().await.add_folder(&window, botpack_location)?;
            message
        }
        downloader::BotpackStatus::Skipped(message) => message,
        downloader::BotpackStatus::RequiresFullDownload => unreachable!(),
    })
}

#[tauri::command]
pub async fn update_bot_pack(window: Window) -> Result<String, BotPackError> {
    let botpack_location = get_content_folder().join(BOTPACK_FOLDER).to_string_lossy().to_string();
    let botpack_status = downloader::update_bot_pack(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location).await;

    Ok(match botpack_status {
        downloader::BotpackStatus::Skipped(message) => message,
        downloader::BotpackStatus::Success(message) => {
            // Configure the folder settings
            BOT_FOLDER_SETTINGS.write().await.add_folder(&window, botpack_location)?;
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, BOTPACK_REPO_OWNER, BOTPACK_REPO_NAME, &botpack_location, true).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.write().await.add_folder(&window, botpack_location)?;
                    message
                }
                downloader::BotpackStatus::Skipped(message) => message,
                downloader::BotpackStatus::RequiresFullDownload => unreachable!(),
            }
        }
    })
}

#[tauri::command]
pub async fn update_map_pack(window: Window) -> Result<String, BotPackError> {
    let mappack_location = get_content_folder().join(MAPPACK_FOLDER);
    let updater = downloader::MapPackUpdater::new(&mappack_location, MAPPACK_REPO.0.to_owned(), MAPPACK_REPO.1.to_owned());
    let location = mappack_location.to_string_lossy().to_string();

    Ok(match updater.needs_update(&window).await {
        downloader::BotpackStatus::Skipped(message) | downloader::BotpackStatus::Success(message) => {
            BOT_FOLDER_SETTINGS.write().await.add_folder(&window, location)?;
            message
        }
        downloader::BotpackStatus::RequiresFullDownload => {
            // We need to download the botpack
            // the most likely cause is the botpack not existing in the first place
            match downloader::download_repo(&window, MAPPACK_REPO.0, MAPPACK_REPO.1, &location, false).await {
                downloader::BotpackStatus::Success(message) => {
                    BOT_FOLDER_SETTINGS.write().await.add_folder(&window, location)?;

                    if updater.get_map_index(&window).await.is_none() {
                        ccprintln(&window, "Error: Couldn't find revision number in map pack");
                        return Err(BotPackError::NoRevision);
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

#[derive(Debug, Error)]
pub enum MatchHandlerError {
    #[error("Couldn't start match handler: {0}")]
    Command(#[from] CommandError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Couldn't find STDIN in match handler")]
    NoStdin,
    #[error("Failed to write to match handler's STDIN")]
    NoWrite,
    #[error("Mutex {0} was poisoned")]
    Poisoned(&'static str),
}

impl_serialize_from_display!(MatchHandlerError);

/// Starts the match handler, which is written in Python so it can use the `RLBot` package (also written in Python)
///
/// Returns None if it fails, otherwise returns pipe for the child process's stdin
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
fn create_match_handler<S: AsRef<OsStr>>(use_pipe: bool, python_path: S) -> Result<(String, (Child, ChildStdin)), MatchHandlerError> {
    let mut child = get_maybe_capture_command(
        &python_path,
        ["-u", "-c", "from rlbot_smh.match_handler import listen; listen(is_raw_json=False)"],
        use_pipe,
    )?
    .stdin(Stdio::piped())
    .spawn()?;

    let stdin = child.stdin.take().ok_or(MatchHandlerError::NoStdin)?;
    Ok((python_path.as_ref().to_string_lossy().to_string(), (child, stdin)))
}

enum CreateHandler {
    /// The bool is whether is not a pipe should be attached to the process
    Yes(bool),
    No,
}

/// Use flate2 to encode a string with gzip then encode the binary with base64 back into a string
fn gzip_encode(s: &str) -> Result<String, MatchHandlerError> {
    let mut e = GzEncoder::new(Vec::new(), Compression::best());
    e.write_all(s.as_bytes())?;
    Ok(BASE64_STANDARD.encode(e.finish()?))
}

/// Send a command to the match handler
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `command` - The command to send to the match handler - can be in multiple parts, for passing arguments
/// * `create_handler` - If the match handler should be started if it's down
fn issue_match_handler_command<S: AsRef<OsStr>>(
    window: &Window,
    command_parts: &[String],
    mut create_handler: CreateHandler,
    python_path: S,
) -> Result<(), MatchHandlerError> {
    let mut command_lock = MATCH_HANDLER_STDIN.lock().map_err(|_| MatchHandlerError::Poisoned("MATCH_HANDLER_STDIN"))?;
    let (used_py_path, match_handler_stdin) = &mut *command_lock;

    if match_handler_stdin.is_none() {
        let CreateHandler::Yes(use_pipe) = create_handler else {
            ccprintln(window, "Not issuing command to handler as it's down and I was told to not start it");
            return Ok(());
        };

        ccprintln(window, "Starting match handler!");
        let (py_path, stdin) = create_match_handler(use_pipe, &python_path)?;

        *match_handler_stdin = Some(stdin);
        *used_py_path = py_path;
        create_handler = CreateHandler::No;
    }

    let command = gzip_encode(&format!("{} | \n", command_parts.join(" | ")))?;
    print!("Issuing command: {command}");
    let (_, stdin) = match_handler_stdin.as_mut().ok_or(MatchHandlerError::NoStdin)?;

    if stdin.write_all(command.as_bytes()).is_err() {
        drop(match_handler_stdin.take());
        if matches!(create_handler, CreateHandler::Yes(_)) {
            ccprintln(window, "Failed to write to match handler, trying to restart...");
            issue_match_handler_command(window, command_parts, create_handler, python_path)
        } else {
            Err(MatchHandlerError::NoWrite)
        }
    } else {
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum MatchInteractionError {
    #[error(transparent)]
    MatchHandler(#[from] MatchHandlerError),
    #[error(transparent)]
    RLNoBot(#[from] setup_manager::RLNoBotError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    MapSetup(#[from] MapSetupError),
}

impl_serialize_from_display!(MatchInteractionError);

/// Perform pre-match startup checks
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
async fn pre_start_match(window: &Window) -> Result<(), MatchInteractionError> {
    let port = gateway_util::find_existing_process(window);
    let rl_is_running = setup_manager::is_rocket_league_running(port.unwrap_or(gateway_util::IDEAL_RLBOT_PORT)).map_err(MatchInteractionError::RLNoBot)?;

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

async fn get_start_match_args_arr(window: &Window, bot_list: Vec<TeamBotBundle>, match_settings: MiniMatchConfig) -> Result<[String; 6], MatchInteractionError> {
    let launcher_settings = LauncherConfig::load(window).await;
    let match_settings = match_settings.setup_for_start_match(&BOT_FOLDER_SETTINGS.read().await.folders)?;

    Ok([
        "start_match".to_owned(),
        serde_json::to_string(&bot_list)?,
        serde_json::to_string(&match_settings)?,
        launcher_settings.preferred_launcher,
        launcher_settings.use_login_tricks.to_string(),
        launcher_settings.rocket_league_exe_path.unwrap_or_default(),
    ])
}

#[tauri::command]
pub async fn get_start_match_arguments(window: Window, bot_list: Vec<TeamBotBundle>, match_settings: MiniMatchConfig) -> Result<String, MatchInteractionError> {
    let raw_string = format!("{} | ", get_start_match_args_arr(&window, bot_list, match_settings).await?.join(" | "));
    println!("Raw JSON command: {raw_string}");
    Ok(gzip_encode(&raw_string)?)
}

/// Starts a match via the match handler with the given settings
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `bot_list` - A list of bots and their settings to use in the match
/// * `match_settings` - The various match settings to use in the match, including scripts (only the path), mutators, game map, etc.
async fn start_match_helper(window: &Window, bot_list: Vec<TeamBotBundle>, match_settings: MiniMatchConfig, use_pipe: bool) -> Result<(), MatchInteractionError> {
    pre_start_match(window).await?;

    let args = get_start_match_args_arr(window, bot_list, match_settings).await?;

    issue_match_handler_command(window, &args, CreateHandler::Yes(use_pipe), &*PYTHON_PATH.read().await)?;

    Ok(())
}

#[tauri::command]
pub async fn start_match(window: Window, bot_list: Vec<TeamBotBundle>, match_settings: MiniMatchConfig) -> Result<(), MatchInteractionError> {
    if let Err(error) = start_match_helper(&window, bot_list, match_settings, USE_PIPE.load(Ordering::Relaxed)).await {
        if let Err(e) = window.emit("match-start-failed", ()) {
            ccprintln!(&window, "Failed to emit match-start-failed: {e}");
        }

        ccprintln(&window, error.to_string());

        Err(error)
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn kill_bots(window: Window) -> Result<(), MatchHandlerError> {
    issue_match_handler_command(&window, &["kill_bots".to_owned()], CreateHandler::No, "")
}

#[tauri::command]
pub async fn shut_down_match_handler() -> Result<(), MatchHandlerError> {
    let mut handler_lock = MATCH_HANDLER_STDIN.lock().map_err(|_| MatchHandlerError::Poisoned("MATCH_HANDLER_STDIN"))?;

    // Send the command to the match handler to shut down
    if let (_, Some((_, stdin))) = &mut *handler_lock {
        const KILL_BOTS_COMMAND: &[u8] = "shut_down | \n".as_bytes();
        stdin.write_all(KILL_BOTS_COMMAND)?;
    }

    // Drop stdin
    handler_lock.1 = None;

    // Wait 15 seconds for the child to exit on it's own, then kill it if it's still running
    if let (_, Some((child, _))) = &mut *handler_lock {
        let start_time = Instant::now();
        let pause_duration = Duration::from_secs_f32(0.25);

        while start_time.elapsed() < Duration::from_secs(15) {
            if let Ok(Some(_)) = child.try_wait() {
                return Ok(());
            }

            thread::sleep(pause_duration);
        }

        child.kill()?;
        child.wait()?;
    }

    Ok(())
}

#[tauri::command]
pub async fn fetch_game_tick_packet_json(window: Window) -> Result<(), MatchHandlerError> {
    issue_match_handler_command(&window, &["fetch_gtp".to_owned()], CreateHandler::No, &*PYTHON_PATH.read().await)
}

#[tauri::command]
pub async fn set_state(window: Window, state: HashMap<String, serde_json::Value>) -> Result<(), MatchInteractionError> {
    Ok(issue_match_handler_command(
        &window,
        &["set_state".to_owned(), serde_json::to_string(&state)?],
        CreateHandler::No,
        &*PYTHON_PATH.read().await,
    )?)
}

#[tauri::command]
pub async fn spawn_car_for_viewing(window: Window, config: BotLooksConfig, team: u8, showcase_type: String, map: String) -> Result<(), MatchInteractionError> {
    let launcher_settings = LauncherConfig::load(&window).await;

    let args = [
        "spawn_car_for_viewing".to_owned(),
        serde_json::to_string(&config)?,
        team.to_string(),
        showcase_type,
        map,
        launcher_settings.preferred_launcher,
        launcher_settings.use_login_tricks.to_string(),
        launcher_settings.rocket_league_exe_path.unwrap_or_default(),
    ];

    Ok(issue_match_handler_command(&window, &args, CreateHandler::Yes(true), &*PYTHON_PATH.read().await)?)
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
/// * `botpack_root` - The path to the root of the `RLBotPack`, which will replace `$RLBOTPACKROOT`
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
/// `botpack_root` - The path to the root of the `RLBotPack`, which will replace `$RLBOTPACKROOT`
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
        skill: player.skill.unwrap_or(1.0),
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
/// `botpack_root` - The path to the root of the `RLBotPack`, which will replace `$RLBOTPACKROOT`
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
/// * `botpack_root` - The path to the root of the `RLBotPack`, which will replace `$RLBOTPACKROOT`
fn make_player_configs(challenge: &Challenge, human_picks: &[String], all_bots: &HashMap<String, Bot>, botpack_root: &Path) -> Vec<TeamBotBundle> {
    let blue = human_picks[..challenge.human_team_size as usize - 1]
        .iter()
        .filter_map(|name| all_bots.get(name))
        .map(|bot| bot_to_team_bot_bundle(bot, Team::Blue, botpack_root));

    let orange = challenge
        .opponent_bots
        .iter()
        .filter_map(|name| all_bots.get(name))
        .map(|bot| bot_to_team_bot_bundle(bot, Team::Orange, botpack_root));

    [make_human_config(Team::Blue)].into_iter().chain(blue).chain(orange).collect()
}

/// Load a script from a Script config
///
/// # Arguments
///
/// * `script` - The JSON map that the key "path" which points to the script's .py file
/// * `botpack_root` - The path to the root of the `RLBotPack`, which will replace `$RLBOTPACKROOT`
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
/// * `botpack_root` - The path to the root of the `RLBotPack`, which will replace `$RLBOTPACKROOT`
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

#[derive(Debug, Error)]
pub enum RunChallengeError {
    #[error(transparent)]
    MatchInteraction(#[from] MatchInteractionError),
    #[error("Could not find challenge with id {0}")]
    NoChallengeId(String),
    #[error("Could not find RLBotPack-master folder")]
    NoBotPackFolder,
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

impl_serialize_from_display!(RunChallengeError);

/// Launch a challenge for the user to play
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `story_save` - The save state of the story, containing all the information about the story
/// * `challenge_id` - The ID of the challenge to run
/// * `picked_teammates` - The teammates that were picked by the human for teammates to use in the challenge
async fn run_challenge(window: &Window, save_state: &StoryState, challenge_id: String, picked_teammates: &[String]) -> Result<(), RunChallengeError> {
    pre_start_match(window).await?;

    let story_settings = save_state.get_story_settings();

    let Some((city, challenge)) = get_challenge_by_id(story_settings, &challenge_id).await else {
        return Err(RunChallengeError::NoChallengeId(challenge_id));
    };

    let all_bots = get_all_bot_configs(story_settings).await;
    let all_scripts = get_all_script_configs(story_settings).await;

    let botpack_root = BOT_FOLDER_SETTINGS
        .read()
        .await
        .folders
        .keys()
        .map(|bf| Path::new(bf).join("RLBotPack-master"))
        .find(|bf| bf.exists())
        .ok_or(RunChallengeError::NoBotPackFolder)?;

    let player_configs = make_player_configs(&challenge, picked_teammates, &all_bots, botpack_root.as_path());
    let match_settings = make_match_config(&challenge, save_state.get_upgrades(), make_script_configs(&challenge, &all_scripts, botpack_root.as_path()));
    let launcher_prefs = LauncherConfig::load(window).await;

    let args = [
        "launch_challenge".to_owned(),
        challenge_id,
        serde_json::to_string(&city.description.color)?,
        serde_json::to_string(&save_state.get_team_settings().color)?,
        serde_json::to_string(&save_state.get_upgrades())?,
        serde_json::to_string(&player_configs)?,
        serde_json::to_string(&match_settings)?,
        serde_json::to_string(&challenge)?,
        serde_json::to_string(&save_state)?,
        launcher_prefs.preferred_launcher,
        launcher_prefs.use_login_tricks.to_string(),
        launcher_prefs.rocket_league_exe_path.unwrap_or_default(),
    ];

    println!("Issuing command: {} | ", args.join(" | "));

    issue_match_handler_command(window, &args, CreateHandler::Yes(true), &*PYTHON_PATH.read().await).map_err(Into::<MatchInteractionError>::into)?;

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
    NoKey(&'static str),
    #[error("Key '{0}' in JSON response was not a string")]
    InvalidKeyType(&'static str),
}

impl_serialize_from_display!(LogUploadError);

/// there are a few references to hastebin in the GUI
/// you should also change those references to avoid user confusion
/// (if changing paste provider)
#[tauri::command]
pub async fn upload_log(window: Window) -> Result<String, LogUploadError> {
    const KEY: &str = "key";

    ccprintln(&window, "Reading log file...");
    let file = AsyncFile::open(get_log_path()).await?;

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).await?;

    if contents.is_empty() {
        return Err(LogUploadError::EmptyLog);
    }

    let (home_folder, replacement_key) = get_home_folder();
    let contents = contents.replace(&home_folder.to_string_lossy().to_string(), replacement_key);

    ccprintln(&window, "Log file read succesfully! Now uploading to hastebin...");

    // send contents to https://hastebin.com/documents via POST
    let res = reqwest::Client::new().post("https://hastebin.com/documents").body(contents).send().await?;

    // the returned JSON looks a little bit like `{"key":"royidegeni"}`
    // Take this and return the key attached to the hastebin URL
    let json: serde_json::Value = res.json().await?;

    let url_postfix = json
        .get(KEY)
        .ok_or_else(|| LogUploadError::NoKey(KEY))?
        .as_str()
        .ok_or_else(|| LogUploadError::InvalidKeyType(KEY))?
        .to_owned();

    let url = format!("https://hastebin.com/{url_postfix}");
    ccprintln!(&window, "Log file uploaded to: {url}");
    Ok(url)
}
