#![allow(clippy::wildcard_imports)]
#![recursion_limit = "256"]

mod bot_management;
mod commands;
mod config_handles;
mod custom_maps;
mod rlbot;
mod settings;
mod stories;
mod tauri_plugin;

#[cfg(windows)]
use registry::{Hive, Security};
#[cfg(windows)]
use std::{os::windows::process::CommandExt, path::Path};

use crate::{
    commands::*,
    config_handles::*,
    settings::{BotFolders, ConsoleTextUpdate, GameTickPacket, StoryConfig, StoryState},
    stories::StoryModeConfig,
};
use crossbeam_channel::{unbounded, SendError, Sender};
use once_cell::sync::Lazy;
use os_pipe::{pipe, PipeWriter};
use serde::Serialize;
use std::{
    collections::HashMap,
    env,
    error::Error as StdError,
    ffi::OsStr,
    fs::{create_dir_all, File, OpenOptions},
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, RwLock,
    },
    thread,
    time::Duration,
};
use tauri::{async_runtime::block_on as tauri_block_on, App, Error as TauriError, Manager, Window};
use thiserror::Error;
use tokio::sync::RwLock as AsyncRwLock;

pub use serde;

const MAIN_WINDOW_NAME: &str = "main";

static NO_CONSOLE_WINDOWS: AtomicBool = AtomicBool::new(true);
static USE_PIPE: AtomicBool = AtomicBool::new(true);
static IS_DEBUG_MODE: AtomicBool = AtomicBool::new(cfg!(debug_assertions));

const BOTPACK_FOLDER: &str = "RLBotPackDeletable";
const MAPPACK_FOLDER: &str = "RLBotMapPackDeletable";
const MAPPACK_REPO: (&str, &str) = ("azeemba", "RLBotMapPack");
const BOTPACK_REPO_OWNER: &str = "RLBot";
const BOTPACK_REPO_NAME: &str = "RLBotPack";
const MAX_CONSOLE_LINES: usize = 840;

static CONSOLE_TEXT: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CONSOLE_INPUT_COMMANDS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CONSOLE_TEXT_EMIT_QUEUE: RwLock<Option<Sender<ConsoleTextUpdate>>> = RwLock::new(None);
static CONSOLE_TEXT_OUT_QUEUE: RwLock<Option<Sender<String>>> = RwLock::new(None);

static MATCH_HANDLER_STDIN: Mutex<(String, Option<(Child, ChildStdin)>)> = Mutex::new((String::new(), None));
static CAPTURE_PIPE_WRITER: Mutex<Option<PipeWriter>> = Mutex::new(None);

static PYTHON_PATH: AsyncRwLock<String> = AsyncRwLock::const_new(String::new());
static CUSTOM_STORIES_CACHE: AsyncRwLock<Lazy<HashMap<StoryConfig, StoryModeConfig>>> = AsyncRwLock::const_new(Lazy::new(HashMap::new));
static BOT_FOLDER_SETTINGS: AsyncRwLock<Lazy<BotFolders>> = AsyncRwLock::const_new(Lazy::new(BotFolders::default));

#[macro_export]
macro_rules! impl_serialize_from_display {
    ($($t:ty),*) => {
        $(
            impl $crate::serde::Serialize for $t {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: $crate::serde::Serializer,
                {
                    serializer.serialize_str(&self.to_string())
                }
            }
        )*
    };
}

#[cfg(windows)]
fn auto_detect_python() -> Option<(String, bool)> {
    let content_folder = get_content_folder();

    let new_python = content_folder.join("Python37\\python.exe");
    if get_command_status(&new_python, ["--version"]) {
        return Some((new_python.to_string_lossy().to_string(), true));
    }

    let old_python = content_folder.join("venv\\Scripts\\python.exe");
    if get_command_status(&old_python, ["--version"]) {
        return Some((old_python.to_string_lossy().to_string(), true));
    }

    // Windows actually doesn't have a python3.7.exe command, just python.exe (no matter what)
    // but there is a pip3.7.exe and stuff
    // we can then use that to find the path to the right python.exe and use that
    for pip in ["pip3.7", "pip3.8", "pip3.9", "pip3.10", "pip3.6", "pip3"] {
        if let Ok(value) = get_python_from_pip(pip) {
            return Some((value, false));
        }
    }

    if get_command_status("python", ["--version"]) {
        Some(("python".to_owned(), false))
    } else {
        None
    }
}

#[cfg(windows)]
#[derive(Debug, Error)]
pub enum WindowsPipLocateError {
    #[error("Couldn't convert stdout to string: {0}")]
    InvalidUTF8(#[from] std::string::FromUtf8Error),
    #[error("{0} has no parent")]
    NoParentError(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("Could not find python.exe")]
    NoPython,
}

#[cfg(windows)]
fn get_python_from_pip(pip: &str) -> Result<String, WindowsPipLocateError> {
    let output = Command::new("where").arg(pip).output()?;
    let stdout = String::from_utf8(output.stdout)?;

    if let Some(first_line) = stdout.lines().next() {
        let python_path = Path::new(first_line)
            .parent()
            .ok_or_else(|| WindowsPipLocateError::NoParentError(first_line.to_owned()))?
            .parent()
            .ok_or_else(|| WindowsPipLocateError::NoParentError(first_line.to_owned()))?
            .join("python.exe");
        if get_command_status(&python_path, ["--version"]) {
            return Ok(python_path.to_string_lossy().to_string());
        }
    }

    Err(WindowsPipLocateError::NoPython)
}

#[cfg(target_os = "macos")]
fn auto_detect_python() -> Option<(String, bool)> {
    for python in ["python3.7", "python3.8", "python3.9", "python3.6", "python3"] {
        if get_command_status(python, ["--version"]) {
            return Some((python.to_owned(), false));
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn auto_detect_python() -> Option<(String, bool)> {
    let content_folder = get_content_folder();
    let rlbot_venv_paths = [content_folder.join("venv/bin/python"), content_folder.join("env/bin/python")];

    for path in &rlbot_venv_paths {
        if get_command_status(path, ["--version"]) {
            return Some((path.to_string_lossy().to_string(), true));
        }
    }

    for python in ["python3.7", "python3.8", "python3.9", "python3.10", "python3.6", "python3"] {
        if get_command_status(python, ["--version"]) {
            return Some((python.to_owned(), false));
        }
    }

    None
}

/// Get the path to the GUI config file
fn get_config_path() -> PathBuf {
    get_content_folder().join("config.ini")
}

/// Get the path to the GUI log file
fn get_log_path() -> PathBuf {
    get_content_folder().join("log.txt")
}

/// Clear the log file
fn clear_log_file() -> IoResult<()> {
    let log_path = get_log_path();

    if !log_path.exists() {
        create_dir_all(log_path.parent().unwrap())?;
    }

    File::create(log_path).map(drop)
}

/// Emits text to the console
/// Also calls println!() to print to the console
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `text` - The text to emit
pub fn ccprintln<T: AsRef<str>>(window: &Window, text: T) {
    emit_text(window, text, false);
}

/// A more convenient way to emit text to the console
/// Similar to the function, but automatically adds calls format!() on the arguments
#[macro_export]
macro_rules! ccprintln {
    ($window:expr) => {
        $crate::ccprintln($window, "")
    };
    ($window:expr, $($arg:tt)*) => {
        $crate::ccprintln($window, format!($($arg)*))
    };
}

/// Emits text to the console, replacing the previous line
/// Also calls println!() to print to the console
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `text` - The text to emit
pub fn ccprintlnr<T: AsRef<str>>(window: &Window, text: T) {
    emit_text(window, text, true);
}

/// A more convenient way to emit text to the console
/// Similar to the function, but automatically adds calls format!() on the arguments
#[macro_export]
macro_rules! ccprintlnr {
    ($window:expr) => {
        $crate::ccprintlnr($window, "")
    };
    ($window:expr, $($arg:tt)*) => {
        $crate::ccprintlnr($window, format!($($arg)*))
    };
}

#[cfg(windows)]
fn has_chrome() -> bool {
    const REG_PATH: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe";

    [Hive::CurrentUser, Hive::LocalMachine]
        .into_iter()
        .filter_map(|install_type| install_type.open(REG_PATH, Security::Read).ok())
        .any(|reg_key| match reg_key.value("") {
            Ok(chrome_path) => Path::new(&chrome_path.to_string()).is_file(),
            Err(_) => false,
        })
}

#[cfg(target_os = "macos")]
fn has_chrome() -> bool {
    get_command_status("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", vec!["--version"])
}

#[cfg(target_os = "linux")]
fn has_chrome() -> bool {
    // google chrome works, but many Linux users especally may prefer to use Chromium instead
    get_command_status("google-chrome", ["--product-version"]) || get_command_status("chromium", ["--product-version"])
}

/// Spawns a process, waits for it to finish, and returns whether or not it completed sucessfully
///
/// # Arguments
///
/// * `program` - The executable to run
/// * `args` - The arguments to pass to the executable
fn get_command_status<S: AsRef<OsStr>, A: AsRef<OsStr>, I: IntoIterator<Item = A>>(program: S, args: I) -> bool {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        // disable window creation
        command.creation_flags(0x0800_0000);
    };

    let Ok(status) = command.args(args).stdout(Stdio::null()).stderr(Stdio::null()).status() else {
        return false;
    };

    status.success()
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Mutex {0} was poisoned")]
    Poisoned(&'static str),
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Pipe is closed")]
    ClosedPipe,
}

/// Returns a Command that, went ran, will have all it's output redirected to the GUI console
/// Be sure to `drop(command)` after spawning the child process! Otherwise a deadlock could happen.
/// This is due to how the `os_pipe` crate works.
///
/// Most of the time, you should try to use `spawn_capture_process()` instead.
///
/// # Errors
///
/// Returns an error when either `CAPTURE_PIPE_WRITER`'s lock is poisoned, or when the capture pipes couldn't be connected.
///
/// # Arguments
///
/// * `program` - The executable to run
/// * `args` - The arguments to pass to the executable
pub fn get_capture_command<S: AsRef<OsStr>, A: AsRef<OsStr>, I: IntoIterator<Item = A>>(program: S, args: I) -> Result<Command, CommandError> {
    let mut command = get_command(program, args);

    let pipe = CAPTURE_PIPE_WRITER.lock().map_err(|_| CommandError::Poisoned("CAPTURE_PIPE_WRITER"))?;
    let out_pipe = pipe.as_ref().ok_or(CommandError::ClosedPipe)?.try_clone()?;
    let err_pipe = pipe.as_ref().ok_or(CommandError::ClosedPipe)?.try_clone()?;

    command.stdout(out_pipe).stderr(err_pipe);

    Ok(command)
}

/// Returns a Command that won't have it's output redirected. Will also tell Windows to not spawn a new console window, and will set the working directory correctly.
///
/// # Errors
///
/// Returns an error when either `CAPTURE_PIPE_WRITER`'s lock is poisoned, or when the capture pipes couldn't be connected.
///
/// # Arguments
///
/// * `program` - The executable to run
/// * `args` - The arguments to pass to the executable
pub fn get_command<S: AsRef<OsStr>, A: AsRef<OsStr>, I: IntoIterator<Item = A>>(program: S, args: I) -> Command {
    let mut command = Command::new(program);
    command.args(args).current_dir(get_content_folder());

    #[cfg(windows)]
    {
        // disable window creation
        command.creation_flags(0x0800_0000);
    }

    command
}

/// Returns a Command that may or may not have it's output redirected. Will also tell Windows to not spawn a new console window (if needed), and will set the working directory correctly.
///
/// # Errors
///
/// Returns an error when either `CAPTURE_PIPE_WRITER`'s lock is poisoned, or when the capture pipes couldn't be connected.
///
/// # Arguments
///
/// * `program` - The executable to run
/// * `args` - The arguments to pass to the executable
pub fn get_maybe_capture_command<S: AsRef<OsStr>, A: AsRef<OsStr>, I: IntoIterator<Item = A>>(program: S, args: I, use_pipe: bool) -> Result<Command, CommandError> {
    if use_pipe {
        get_capture_command(program, args)
    } else {
        let mut command = Command::new(program);
        command.args(args).current_dir(get_content_folder());
        Ok(command)
    }
}

/// Spawns a process that will have it's output captured and sent to the GUI console.
/// This function is esstential because is drops the command, which avoids a deadlock.
///
/// Note: Child != Command
///
/// # Errors
///
/// Returns an error when the child process fails to start.
///
/// # Arguments
///
/// * `program` - The executable to run
/// * `args` - The arguments to pass to the executable
pub fn spawn_capture_process<S: AsRef<OsStr>, A: AsRef<OsStr>, I: IntoIterator<Item = A>>(program: S, args: I) -> Result<Child, CommandError> {
    Ok(get_capture_command(program, args)?.spawn()?)
}

/// Spawns a process that will have it's output captured and sent to the GUI console.
/// Wait for the process to exit, and returns the exit code.
///
///  Returns 2 if the process failed to start, and 1 if we failed to get the exit code but at least something happened.
///
/// # Arguments
///
/// * `program` - The executable to run
/// * `args` - The arguments to pass to the executable
pub fn spawn_capture_process_and_get_exit_code<S: AsRef<OsStr>, A: AsRef<OsStr>, I: IntoIterator<Item = A>>(program: S, args: I) -> i32 {
    let Ok(mut child) = spawn_capture_process(program, args) else {
        return 2;
    };

    let Ok(exit_status) = child.wait() else {
        return 2;
    };

    exit_status.code().unwrap_or(1)
}

/// Check whether or not the rlbot pip package is installed
///
/// # Errors
///
/// This function will return an error if `PYTHON_PATH`'s lock has been poisoned.
pub async fn check_has_rlbot() -> bool {
    get_command_status(&*PYTHON_PATH.read().await, ["-c", "import rlbot"])
}

#[cfg(windows)]
fn get_content_folder() -> PathBuf {
    PathBuf::from(format!("{}\\RLBotGUIX", env::var("LOCALAPPDATA").unwrap()))
}

#[cfg(target_os = "macos")]
fn get_content_folder() -> PathBuf {
    get_home_folder().0.join("Library/Application Support/rlbotgui")
}

#[cfg(target_os = "linux")]
fn get_content_folder() -> PathBuf {
    get_home_folder().0.join(".RLBotGUI")
}

#[cfg(windows)]
fn get_home_folder() -> (PathBuf, &'static str) {
    (PathBuf::from(env::var("USERPROFILE").unwrap()), "%USERPROFILE%")
}

#[cfg(not(windows))]
fn get_home_folder() -> (PathBuf, &'static str) {
    (PathBuf::from(env::var("HOME").unwrap()), "~")
}

#[derive(Debug, Error)]
pub enum InternalConsoleError {
    #[error("Mutex {0} was poisoned")]
    Poisoned(&'static str),
    #[error("Could not complete I/O operation: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    AnsiToHTML(#[from] ansi_to_html::Error),
    #[error(transparent)]
    Tauri(#[from] TauriError),
    #[error("{0} was None")]
    None(&'static str),
    #[error(transparent)]
    ConsoleUpdateSender(#[from] SendError<ConsoleTextUpdate>),
    #[error(transparent)]
    ConsoleWriterSender(#[from] SendError<String>),
}

fn write_console_text_out_queue_to_file(window: &Window, to_write_out: Vec<String>) -> Result<(), InternalConsoleError> {
    let mut file = OpenOptions::new().write(true).append(true).open(get_log_path())?;
    for line in to_write_out {
        if let Err(e) = writeln!(file, "{line}") {
            ccprintln!(window, "Error writing to log file: {e}");
        }
    }

    Ok(())
}

fn update_internal_console(update: &ConsoleTextUpdate) -> Result<(), InternalConsoleError> {
    let mut console_text = CONSOLE_TEXT.lock().map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT"))?;
    if update.replace_last {
        console_text.pop();
    }
    console_text.push(update.content.clone());

    if console_text.len() > MAX_CONSOLE_LINES {
        console_text.remove(0);
    }

    Ok(())
}

fn try_emit_signal<S: Serialize + Clone>(window: &Window, signal: &str, payload: S) -> (String, Option<TauriError>) {
    (signal.to_owned(), window.emit(signal, payload).err())
}

fn emit_console_text_emit_queue(window: &Window, mut updates: Vec<ConsoleTextUpdate>) -> Result<(), InternalConsoleError> {
    if updates.is_empty() {
        return Ok(());
    }

    // If an update is replace_last, then remove the previous update.
    let mut i = 1;
    while i < updates.len() {
        if updates[i].replace_last {
            updates[i].replace_last = updates[i - 1].replace_last;
            updates.remove(i - 1);
        } else {
            i += 1;
        }
    }

    window.emit("new-console-texts", updates)?;

    Ok(())
}

fn issue_console_update(text: String, replace_last: bool) -> Result<(), InternalConsoleError> {
    println!("{text}");

    let converted_and_escaped = ansi_to_html::convert_escaped(&text)?;
    let update = ConsoleTextUpdate::from(converted_and_escaped, replace_last);
    update_internal_console(&update)?;

    CONSOLE_TEXT_EMIT_QUEUE
        .read()
        .map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT_EMIT_QUEUE"))?
        .as_ref()
        .ok_or_else(|| InternalConsoleError::None("CONSOLE_TEXT_EMIT_QUEUE"))?
        .send(update)?;

    CONSOLE_TEXT_OUT_QUEUE
        .read()
        .map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT_OUT_QUEUE"))?
        .as_ref()
        .ok_or_else(|| InternalConsoleError::None("CONSOLE_TEXT_OUT_QUEUE"))?
        .send(text)?;

    Ok(())
}

fn try_emit_text<T: AsRef<str>>(window: &Window, text: T, replace_last: bool) -> (String, Option<TauriError>) {
    let text = text.as_ref();
    if text == "-|-*|MATCH START FAILED|*-|-" {
        eprintln!("START MATCH FAILED");
        try_emit_signal(window, "match-start-failed", ())
    } else if text == "-|-*|MATCH STARTED|*-|-" {
        println!("MATCH STARTED");
        try_emit_signal(window, "match-started", ())
    } else if text.starts_with("-|-*|GTP ") && text.ends_with("|*-|-") {
        let text = text.replace("-|-*|GTP ", "").replace("|*-|-", "");
        let gtp: GameTickPacket = serde_json::from_str(&text).unwrap();
        try_emit_signal(window, "gtp", gtp)
    } else if text.starts_with("-|-*|STORY_RESULT ") && text.ends_with("|*-|-") {
        println!("GOT STORY RESULT {text}");
        let text = text.replace("-|-*|STORY_RESULT ", "").replace("|*-|-", "");
        let save_state: StoryState = serde_json::from_str(&text).unwrap();
        save_state.save_sync(window);
        try_emit_signal(window, "load_updated_save_state", save_state)
    } else {
        if let Err(e) = issue_console_update(text.to_owned(), replace_last) {
            ccprintln(window, e.to_string());
        }

        Default::default()
    }
}

fn emit_text<T: AsRef<str>>(window: &Window, text: T, replace_last: bool) {
    if let (signal, Some(e)) = try_emit_text(window, text, replace_last) {
        ccprintln!(window, "Error emitting {signal}: {e}");
    }
}

fn gui_setup_load_config(window: &Window) {
    tauri_block_on(async {
        let gui_config = load_gui_config(window).await;
        *PYTHON_PATH.write().await = gui_config.get("python_config", "path").unwrap_or_else(|| auto_detect_python().unwrap_or_default().0);
        **BOT_FOLDER_SETTINGS.write().await = BotFolders::load_from_conf(&load_gui_config(window).await);
    });
}

fn gui_setup(app: &mut App) -> Result<(), Box<dyn StdError>> {
    let window = app.get_window(MAIN_WINDOW_NAME).ok_or(format!("Cannot find window '{MAIN_WINDOW_NAME}'"))?;
    let window2 = window.clone();
    let window3 = window.clone();
    let window4 = window.clone();

    let (emit_sender, emit_receiver) = unbounded();
    CONSOLE_TEXT_EMIT_QUEUE
        .write()
        .map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT_EMIT_QUEUE"))?
        .replace(emit_sender);

    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs_f32(1. / 60.));
        let updates = emit_receiver.try_iter().collect();

        if let Err(e) = emit_console_text_emit_queue(&window3, updates) {
            ccprintln(&window3, e.to_string());
        }
    });

    let (file_write_sender, file_write_receiver) = unbounded();
    CONSOLE_TEXT_OUT_QUEUE
        .write()
        .map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT_OUT_QUEUE"))?
        .replace(file_write_sender);

    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs_f32(1. / 3.));
        let to_write_out = file_write_receiver.try_iter().collect();
        if let Err(e) = write_console_text_out_queue_to_file(&window4, to_write_out) {
            ccprintln(&window2, e.to_string());
        }
    });

    clear_log_file()?;
    gui_setup_load_config(&window);

    let (mut pipe_reader, pipe_writer) = pipe()?;
    *CAPTURE_PIPE_WRITER.lock()? = Some(pipe_writer);

    thread::spawn(move || {
        let mut next_replace_last = false;
        loop {
            let mut text = String::new();
            let mut will_replace_last = next_replace_last;
            next_replace_last = false;

            loop {
                let mut buf = [0];
                match pipe_reader.read(&mut buf[..]) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let string = String::from_utf8_lossy(&buf).clone();
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

            emit_text(&window, text, will_replace_last);
        }
    });

    Ok(())
}

#[tauri::command]
fn is_debug_build() -> bool {
    IS_DEBUG_MODE.load(Ordering::Relaxed)
}

fn main() {
    let use_pipe = !std::env::args().any(|arg| arg == "--no-pipe");
    USE_PIPE.store(use_pipe, Ordering::Relaxed);

    let no_console_windows = !std::env::args().any(|arg| arg == "--console");
    NO_CONSOLE_WINDOWS.store(no_console_windows, Ordering::Relaxed);

    #[cfg(all(not(debug_assertions), windows))]
    if use_pipe && no_console_windows {
        unsafe {
            winapi::um::wincon::FreeConsole();
        }
    }

    if std::env::args().any(|arg| arg == "--debug") {
        IS_DEBUG_MODE.store(true, Ordering::Relaxed);
    }

    println!("Config path: {}", get_config_path().display());

    tauri::Builder::default()
        .setup(|app| gui_setup(app))
        .plugin(tauri_plugin::init())
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
            get_console_input_commands,
            get_detected_python_path,
            get_missing_bot_packages,
            get_missing_script_packages,
            get_missing_bot_logos,
            get_missing_script_logos,
            is_windows,
            install_python,
            download_bot_pack,
            update_bot_pack,
            is_botpack_up_to_date,
            check_rlbot_python,
            update_map_pack,
            start_match,
            get_launcher_settings,
            save_launcher_settings,
            kill_bots,
            fetch_game_tick_packet_json,
            set_state,
            spawn_car_for_viewing,
            get_downloaded_botpack_commit_id,
            story_load_save,
            story_new_save,
            get_story_settings,
            get_map_pack_revision,
            get_cities_json,
            pick_json_file,
            get_bots_configs,
            story_delete_save,
            launch_challenge,
            story_save_state,
            purchase_upgrade,
            recruit,
            is_debug_build,
            run_command,
            upload_log,
            create_python_venv,
            get_selected_tab,
            set_selected_tab,
            shut_down_match_handler,
            get_start_match_arguments,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
