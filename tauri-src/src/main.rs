#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod bot_management;
mod commands;
mod config_handles;
mod custom_maps;
mod rlbot;
mod settings;

use crate::commands::*;
use crate::config_handles::*;
use crate::settings::*;
use lazy_static::{initialize, lazy_static};
use os_pipe::{pipe, PipeWriter};
use std::io;
use std::process::Child;
use std::process::ChildStdin;
use std::sync::Mutex;
use std::{
    env,
    ffi::OsStr,
    fs::{create_dir_all, write},
    io::Read,
    ops::Not,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};
use tauri::Manager;
use tauri::Menu;

const BOTPACK_FOLDER: &str = "RLBotPackDeletable";
const MAPPACK_FOLDER: &str = "RLBotMapPackDeletable";
const MAPPACK_REPO: (&str, &str) = ("azeemba", "RLBotMapPack");
const BOTPACK_REPO_OWNER: &str = "RLBot";
const BOTPACK_REPO_NAME: &str = "RLBotPack";

lazy_static! {
    static ref BOT_FOLDER_SETTINGS: Mutex<BotFolderSettings> = Mutex::new(BotFolderSettings::new());
    static ref MATCH_SETTINGS: Mutex<MatchSettings> = Mutex::new(MatchSettings::new());
    static ref PYTHON_PATH: Mutex<String> = Mutex::new(load_gui_config().get("python_config", "path").unwrap_or_else(|| auto_detect_python().unwrap_or_default()));
    static ref CONSOLE_TEXT: Mutex<Vec<ConsoleText>> = Mutex::new(vec![
        ConsoleText::from("Welcome to the RLBot Console!".to_string(), false),
        ConsoleText::from("".to_string(), false)
    ]);
    static ref MATCH_HANDLER_STDIN: Mutex<Option<ChildStdin>> = Mutex::new(None);
    static ref CAPTURE_PIPE_WRITER: Mutex<Option<PipeWriter>> = Mutex::new(None);
}

#[cfg(windows)]
fn auto_detect_python() -> Option<String> {
    let content_folder = get_content_folder();

    match content_folder.join("Python37\\python.exe") {
        path if path.exists() => Some(path.to_string_lossy().to_string()),
        _ => match content_folder.join("venv\\Scripts\\python.exe") {
            path if path.exists() => Some(path.to_string_lossy().to_string()),
            _ => {
                // Windows actually doesn't have a python3.7.exe command, just python.exe (no matter what)
                // but there is a pip3.7.exe and stuff
                // we can then use that to find the path to the right python.exe and use that
                for pip in ["pip3.7", "pip3.8", "pip3.6", "pip3"] {
                    if let Ok(value) = get_python_from_pip(pip) {
                        return Some(value);
                    }
                }

                if get_command_status("python", vec!["--version"]) {
                    Some("python".to_string())
                } else {
                    None
                }
            }
        },
    }
}

#[cfg(windows)]
use std::error::Error;

#[cfg(windows)]
fn get_python_from_pip(pip: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("where").arg(pip).output()?;
    let stdout = String::from_utf8(output.stdout)?;

    if let Some(first_line) = stdout.lines().next() {
        let python_path = Path::new(first_line).parent().unwrap().parent().unwrap().join("python.exe");
        if python_path.exists() {
            return Ok(python_path.to_string_lossy().to_string());
        }
    }

    Err("Could not find python.exe".into())
}

#[cfg(target_os = "macos")]
fn auto_detect_python() -> Option<String> {
    for python in ["python3.7", "python3.8", "python3.6", "python3"] {
        if get_command_status(python, vec!["--version"]) {
            return Some(python.to_string());
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn auto_detect_python() -> Option<String> {
    match get_content_folder().join("env/bin/python") {
        path if path.exists() => Some(path.to_string_lossy().to_string()),
        _ => {
            for python in ["python3.7", "python3.8", "python3.6", "python3"] {
                if get_command_status(python, vec!["--version"]) {
                    return Some(python.to_string());
                }
            }

            None
        }
    }
}

fn get_config_path() -> PathBuf {
    get_content_folder().join("config.ini")
}

pub fn ccprintln(text: String) {
    println!("{}", &text);
    CONSOLE_TEXT.lock().unwrap().push(ConsoleText::from(text, false));
}

pub fn ccprintlnr(text: String) {
    println!("\r{}", &text);
    let mut ct = CONSOLE_TEXT.lock().unwrap();
    ct.pop();
    ct.push(ConsoleText::from(text, false));
}

pub fn ccprintlne(text: String) {
    eprintln!("{}", &text);
    CONSOLE_TEXT.lock().unwrap().push(ConsoleText::from(text, true));
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

/// Be sure to drop(command) after spawning the child process! Otherwise a deadlock could happen.
/// This is due to how the os_pipe crate works.
pub fn get_capture_command<S: AsRef<OsStr>>(program: S, args: &[&str]) -> Command {
    let mut command = Command::new(program);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // disable window creation
        command.creation_flags(0x08000000);
    };

    let pipe = CAPTURE_PIPE_WRITER.lock().unwrap();
    let out_pipe = pipe.as_ref().unwrap().try_clone().unwrap();
    let err_pipe = pipe.as_ref().unwrap().try_clone().unwrap();

    command.args(args).stdout(out_pipe).stderr(err_pipe);

    command
}

/// This function is esstential because is drops the command, which causes a deadlock
/// Note: Child != Command
pub fn spawn_capture_process<S: AsRef<OsStr>>(program: S, args: &[&str]) -> io::Result<Child> {
    get_capture_command(program, args).spawn()
}

pub fn spawn_capture_process_and_get_exit_code<S: AsRef<OsStr>>(program: S, args: &[&str]) -> i32 {
    if let Ok(mut child) = spawn_capture_process(program, args) {
        child.wait().unwrap().code().unwrap_or(1)
    } else {
        2
    }
}

pub fn check_has_rlbot() -> bool {
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

fn bootstrap_python_script<T: AsRef<Path>, C: AsRef<[u8]>>(content_folder: T, file_name: &str, file_contents: C) {
    let full_path = content_folder.as_ref().join(file_name);
    println!("{}: {}", file_name, full_path.to_string_lossy());

    if !full_path.parent().unwrap().exists() {
        create_dir_all(&full_path).unwrap();
    }

    write(full_path, file_contents).unwrap();
}

fn main() {
    println!("Config path: {}", get_config_path().display());
    load_gui_config();

    let content_folder = get_content_folder();
    bootstrap_python_script(&content_folder, "get_missing_packages.py", include_str!("get_missing_packages.py"));
    bootstrap_python_script(&content_folder, "match_handler.py", include_str!("match_handler.py"));

    initialize(&BOT_FOLDER_SETTINGS);
    initialize(&MATCH_SETTINGS);
    initialize(&PYTHON_PATH);
    initialize(&CONSOLE_TEXT);
    initialize(&MATCH_HANDLER_STDIN);
    initialize(&CAPTURE_PIPE_WRITER);

    let mut app = tauri::Builder::default();

    if cfg!(target_os = "macos") {
        // Only used in MacOS because copy/pasting and stuff won't work otherwise
        // Also, MacOS is the only OS with some actually slick app menu integration
        // Might as well add it encase MacOS support gets added in the future
        app = app.menu(Menu::os_default("RLBotGUI"));
    }

    app.setup(|app| {
        let main_window = app.get_window("main").unwrap();
        let (mut pipe_reader, pipe_writer) = pipe().unwrap();
        *CAPTURE_PIPE_WRITER.lock().unwrap() = Some(pipe_writer);

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

                if text == "-|-*|MATCH START FAILED|*-|-" {
                    eprintln!("START MATCH FAILED");
                    main_window.emit("match-start-failed", ()).unwrap();
                } else if text == "-|-*|MATCH STARTED|*-|-" {
                    eprintln!("MATCH STARTED");
                    main_window.emit("match-started", ()).unwrap();
                } else if let Some(update) = text.is_empty().not().then(|| ConsoleTextUpdate::from(text, false, will_replace_last)) {
                    let mut console_text = CONSOLE_TEXT.lock().unwrap();
                    if update.replace_last {
                        console_text.pop();
                    }
                    console_text.push(update.content.clone());

                    if console_text.len() > 1200 {
                        console_text.drain(1200..);
                    }

                    main_window.emit("new-console-text", vec![update]).unwrap();
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
        update_bot_pack,
        is_botpack_up_to_date,
        check_rlbot_python,
        update_map_pack,
        start_match,
        get_launcher_settings,
        save_launcher_settings,
        kill_bots,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
