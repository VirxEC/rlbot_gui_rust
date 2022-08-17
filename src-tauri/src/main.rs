#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
#![allow(clippy::items_after_statements, clippy::wildcard_imports, clippy::unused_async)]
#![forbid(unsafe_code)]

mod bot_management;
mod commands;
mod config_handles;
mod configparser;
mod custom_maps;
mod is_online;
mod rlbot;
mod settings;
mod stories;

use crate::{
    commands::*,
    config_handles::*,
    settings::{BotFolders, ConsoleTextUpdate, GameTickPacket, StoryConfig, StoryState},
};
use lazy_static::lazy_static;
use os_pipe::{pipe, PipeWriter};
#[cfg(windows)]
use registry::{Hive, Security};
use serde::Serialize;
#[cfg(windows)]
use std::path::Path;
use std::{
    collections::HashMap,
    env,
    error::Error as StdError,
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    string::FromUtf8Error,
    sync::Mutex,
    thread,
    time::Duration,
};
use tauri::{App, Error as TauriError, Manager, Window};
use thiserror::Error;
use tokio::sync::Mutex as AsyncMutex;

const BOTPACK_FOLDER: &str = "RLBotPackDeletable";
const MAPPACK_FOLDER: &str = "RLBotMapPackDeletable";
const MAPPACK_REPO: (&str, &str) = ("azeemba", "RLBotMapPack");
const BOTPACK_REPO_OWNER: &str = "RLBot";
const BOTPACK_REPO_NAME: &str = "RLBotPack";
const MAX_CONSOLE_LINES: usize = 840;

static CONSOLE_TEXT: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CONSOLE_TEXT_OUT_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CONSOLE_INPUT_COMMANDS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static PYTHON_PATH: Mutex<String> = Mutex::new(String::new());
static BOT_FOLDER_SETTINGS: Mutex<Option<BotFolders>> = Mutex::new(None);
static MATCH_HANDLER_STDIN: Mutex<Option<ChildStdin>> = Mutex::new(None);
static CAPTURE_PIPE_WRITER: Mutex<Option<PipeWriter>> = Mutex::new(None);

lazy_static! {
    static ref BOTS_BASE: AsyncMutex<Option<JsonMap>> = AsyncMutex::new(None);
    static ref STORIES_CACHE: AsyncMutex<HashMap<StoryConfig, JsonMap>> = AsyncMutex::new(HashMap::new());
}

#[cfg(windows)]
fn auto_detect_python() -> Option<(String, bool)> {
    let content_folder = get_content_folder();

    let new_python = content_folder.join("Python37\\python.exe");
    if new_python.exists() {
        return Some((new_python.to_string_lossy().to_string(), true));
    }

    let old_python = content_folder.join("venv\\Scripts\\python.exe");
    if old_python.exists() {
        return Some((old_python.to_string_lossy().to_string(), true));
    }

    // Windows actually doesn't have a python3.7.exe command, just python.exe (no matter what)
    // but there is a pip3.7.exe and stuff
    // we can then use that to find the path to the right python.exe and use that
    for pip in ["pip3.7", "pip3.8", "pip3.9", "pip3.6", "pip3"] {
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

#[derive(Debug, Error)]
pub enum WindowsPipLocateError {
    #[error("Couldn't convert stdout to string: {0}")]
    InvalidUTF8(#[from] FromUtf8Error),
    #[error("{0} has no parent")]
    NoParentError(String),
    #[error("I/O error: {0}")]
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
        if python_path.exists() {
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
    let path = get_content_folder().join("env/bin/python");
    if path.exists() {
        return Some((path.to_string_lossy().to_string(), true));
    }

    for python in ["python3.7", "python3.8", "python3.9", "python3.6", "python3"] {
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
    File::create(get_log_path()).map(drop)
}

/// Emits text to the console
/// Also calls println!() to print to the console
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `text` - The text to emit
pub fn ccprintln<T: AsRef<str>>(window: &Window, text: T) {
    let text = text.as_ref();
    println!("{}", text);
    emit_text(window, text, false);
}

/// Emits text to the console, replacing the previous line
/// Also calls println!() to print to the console
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `text` - The text to emit
pub fn ccprintlnr(window: &Window, text: String) {
    println!("{}", &text);
    emit_text(window, text, true);
}

/// Emits text to the console
/// Also calls printlne!() to print to the console
///
/// # Arguments
///
/// * `window` - A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `text` - The text to emit
pub fn ccprintlne(window: &Window, text: String) {
    eprintln!("{}", &text);
    emit_text(window, text, false);
}

#[cfg(windows)]
fn has_chrome() -> bool {
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
        use std::os::windows::process::CommandExt;
        // disable window creation
        command.creation_flags(0x08000000);
    };

    match command.args(args).stdout(Stdio::null()).stderr(Stdio::null()).status() {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Mutex {0} was poisoned")]
    Poisoned(String),
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
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // disable window creation
        command.creation_flags(0x08000000);
    };

    let pipe = CAPTURE_PIPE_WRITER.lock().map_err(|_| CommandError::Poisoned("CAPTURE_PIPE_WRITER".to_owned()))?;
    let out_pipe = pipe.as_ref().ok_or(CommandError::ClosedPipe)?.try_clone()?;
    let err_pipe = pipe.as_ref().ok_or(CommandError::ClosedPipe)?.try_clone()?;

    command.args(args).current_dir(get_content_folder()).stdout(out_pipe).stderr(err_pipe);

    Ok(command)
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
    if let Ok(mut child) = spawn_capture_process(program, args) {
        if let Ok(exit_status) = child.wait() {
            return exit_status.code().unwrap_or(1);
        }
    }

    2
}

/// Check whether or not the rlbot pip package is installed
///
/// # Errors
///
/// This function will return an error if `PYTHON_PATH`'s lock has been poisoned.
pub fn check_has_rlbot() -> Result<bool, String> {
    Ok(get_command_status(&*PYTHON_PATH.lock().map_err(|err| err.to_string())?, ["-c", "import rlbot"]))
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

#[cfg(windows)]
fn get_home_folder() -> (PathBuf, &'static str) {
    (PathBuf::from(env::var("USERPROFILE").unwrap()), "%USERPROFILE%")
}

#[cfg(not(windows))]
fn get_home_folder() -> (PathBuf, &'static str) {
    (PathBuf::from(env::var("HOME").unwrap()), "~")
}

#[derive(Debug, Error)]
enum InternalConsoleError {
    #[error("Mutex {0} was poisoned")]
    Poisoned(String),
    #[error("Could not complete I/O operation: {0}")]
    Io(#[from] std::io::Error),
}

fn write_console_text_out_queue_to_file() -> Result<(), InternalConsoleError> {
    let mut queue = CONSOLE_TEXT_OUT_QUEUE
        .lock()
        .map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT_OUT_QUEUE".to_owned()))?;

    if queue.is_empty() {
        return Ok(());
    }

    let to_write_out = queue.drain(..).collect::<Vec<_>>();
    drop(queue);

    let mut file = OpenOptions::new().write(true).append(true).open(get_log_path())?;

    for line in to_write_out {
        writeln!(file, "{line}")?;
    }

    Ok(())
}

fn update_internal_console(update: &ConsoleTextUpdate) -> Result<(), InternalConsoleError> {
    let mut console_text = CONSOLE_TEXT.lock().map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT".to_owned()))?;
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

fn issue_console_update(window: &Window, text: String, replace_last: bool) -> (String, Option<TauriError>) {
    match CONSOLE_TEXT_OUT_QUEUE.lock() {
        Ok(mut ctoq) => ctoq.push(text.clone()),
        Err(_) => ccprintlne(window, "Mutex CONSOLE_TEXT_OUT_QUEUE is poisoned".to_owned()),
    }

    match ansi_to_html::convert_escaped(&text) {
        Ok(converted_and_escaped) => {
            let update = ConsoleTextUpdate::from(converted_and_escaped, replace_last);
            if let Err(e) = update_internal_console(&update) {
                ccprintlne(window, e.to_string());
            }
            try_emit_signal(window, "new-console-text", update)
        }
        Err(e) => {
            ccprintlne(window, e.to_string());
            Default::default()
        }
    }
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
        println!("GOT STORY RESULT");
        let text = text.replace("-|-*|STORY_RESULT ", "").replace("|*-|-", "");
        let save_state: StoryState = serde_json::from_str(&text).unwrap();
        save_state.save(window);
        try_emit_signal(window, "load_updated_save_state", save_state)
    } else {
        issue_console_update(window, text.to_owned(), replace_last)
    }
}

fn emit_text<T: AsRef<str>>(window: &Window, text: T, replace_last: bool) {
    let (signal, error) = try_emit_text(window, text, replace_last);
    if let Some(e) = error {
        ccprintlne(window, format!("Error emitting {signal}: {e}"));
    }
}

fn gui_setup_load_config(window: &Window) -> Result<(), Box<dyn StdError>> {
    let gui_config = load_gui_config(window);
    *PYTHON_PATH.lock()? = gui_config.get("python_config", "path").unwrap_or_else(|| auto_detect_python().unwrap_or_default().0);
    *BOT_FOLDER_SETTINGS.lock()? = Some(BotFolders::load_from_conf(&gui_config));
    Ok(())
}

fn gui_setup(app: &mut App) -> Result<(), Box<dyn StdError>> {
    const MAIN_WINDOW_NAME: &str = "main";
    let window = app.get_window(MAIN_WINDOW_NAME).ok_or(format!("Cannot find window '{MAIN_WINDOW_NAME}'"))?;
    let window2 = window.clone();

    clear_log_file()?;

    gui_setup_load_config(&window)?;

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

    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs_f32(1. / 5.));
        if let Err(e) = write_console_text_out_queue_to_file() {
            ccprintlne(&window2, e.to_string());
        }
    });

    Ok(())
}

#[tauri::command]
fn is_debug_build() -> bool {
    cfg!(debug_assertions)
}

fn main() {
    println!("Config path: {}", get_config_path().display());

    tauri::Builder::default()
        .setup(|app| gui_setup(app))
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
