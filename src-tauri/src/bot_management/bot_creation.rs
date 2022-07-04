use super::{
    cfg_helper::{change_key_in_cfg, load_cfg, save_cfg},
    zip_extract_fixed,
};
use crate::{
    ccprintln, ccprintlne,
    rlbot::parsing::{
        bot_config_bundle::{BOT_CONFIG_MODULE_HEADER, BOT_CONFIG_PARAMS_HEADER, EXECUTABLE_PATH_KEY, NAME_KEY},
        directory_scanner::scan_directory_for_bot_configs,
    },
    BOT_FOLDER_SETTINGS,
};
use fs_extra::dir::{move_dir, CopyOptions};
use rand::Rng;
use regex::{Regex, Replacer};
use reqwest::IntoUrl;
use sanitize_filename::sanitize;
use std::{
    collections::hash_map::DefaultHasher,
    fs::{remove_file, rename, write, File},
    hash::{Hash, Hasher},
    io::{Cursor, Read, Result as IoResult},
    path::{Path, PathBuf},
};
use tauri::Window;

pub const CREATED_BOTS_FOLDER: &str = "MyBots";

/// Downloads a ZIP from a given URL and unpacks it to top_dir, updating progress in the window along the way
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `url`: The URL of the ZIP that should be downloaded
/// * `top_dir`: The path to the folder where the ZIP will get extracted
async fn download_extract_bot_template<T: IntoUrl>(window: &Window, url: T, top_dir: &Path) -> Result<(), String> {
    match reqwest::get(url).await {
        Ok(res) => {
            let bytes = match res.bytes().await {
                Ok(bytes) => bytes,
                Err(e) => {
                    return Err(format!("Failed to download the bot template: {}", e));
                }
            };

            if let Err(e) = zip_extract_fixed::extract(window, Cursor::new(bytes), top_dir, true, true) {
                return Err(format!("Failed to extract zip: {}", e));
            }
        }
        Err(e) => {
            return Err(format!("Failed to download bot template: {}", e));
        }
    }

    Ok(())
}

/// Download and setup a new Python bot
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `bot_name`: The name of the bot
/// * `directory`: The base directory to put the bot it, which must exist already
pub async fn bootstrap_python_bot(window: &Window, bot_name: String, directory: PathBuf) -> Result<String, String> {
    let sanitized_name = sanitize(&bot_name);
    let top_dir = directory.join(&sanitized_name);

    if top_dir.exists() {
        return Err(format!("There is already a bot named {}, please choose a different name!", sanitized_name));
    }

    download_extract_bot_template(window, "https://github.com/RLBot/RLBotPythonExample/archive/master.zip", top_dir.as_path()).await?;

    let bundles = scan_directory_for_bot_configs(&top_dir.to_string_lossy());
    let bundle = bundles.iter().next().unwrap();
    let config_file = bundle.path.clone().unwrap();
    let python_file = bundle.python_path.clone().unwrap();

    change_key_in_cfg(&config_file, BOT_CONFIG_MODULE_HEADER, NAME_KEY, bot_name)?;

    BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_file(window, config_file.clone());

    if open::that(python_file).is_err() {
        // We don't want to return an error here, because the bot was successfully created
        ccprintlne(
            window,
            format!("You have no default program to open .py files. Your new bot is located at {}", top_dir.to_string_lossy()),
        );
    }

    Ok(config_file)
}

/// Load a file, replace all the matching regex with the replacement, and save the file - returns potential IO errors
///
/// # Arguments
///
/// * `file_path`: Path to the file that needs to be edited
/// * `regex`: The regex that should be matched
/// * `replacement`: The string that should replace everything that matches `regex`
fn replace_all_regex_in_file<R: Replacer>(file_path: &Path, regex: &Regex, replacement: R) -> IoResult<()> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    write(file_path, regex.replace_all(&contents, replacement).as_bytes())?;

    Ok(())
}

/// Download and setup a new Python hivemind-style bot
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `hive_name`: The name of the bots
/// * `directory`: The base directory to put the bot it, which must exist already
pub async fn bootstrap_python_hivemind(window: &Window, hive_name: String, directory: PathBuf) -> Result<String, String> {
    let sanitized_name = sanitize(&hive_name);
    let top_dir = directory.join(&sanitized_name);

    if top_dir.exists() {
        return Err(format!("There is already a bot named {}, please choose a different name!", sanitized_name));
    }

    download_extract_bot_template(window, "https://github.com/RLBot/RLBotPythonHivemindExample/archive/master.zip", top_dir.as_path()).await?;

    let config_file = top_dir.join("config.cfg");
    let drone_file = top_dir.join("src").join("drone.py");
    let hive_file = top_dir.join("src").join("hive.py");

    change_key_in_cfg(&config_file, BOT_CONFIG_MODULE_HEADER, NAME_KEY, hive_name.clone())?;

    if let Err(e) = replace_all_regex_in_file(&drone_file, &Regex::new(r"hive_name = .*$").unwrap(), format!("hive_name = \"{} Hivemind\"", &hive_name)) {
        return Err(format!("Failed to replace hivemind drone name: {}", e));
    }

    let mut hasher = DefaultHasher::new();
    hive_name.hash(&mut hasher);
    let mut hive_key = hasher.finish();
    // add random number between 100000 and 999999 to hive_id
    hive_key += rand::random::<u64>() % 1000000;

    if let Err(e) = replace_all_regex_in_file(&drone_file, &Regex::new(r"hive_key = .*$").unwrap(), format!("hive_key = \"{}\"", hive_key)) {
        return Err(format!("Failed to replace hive_key in drone.py: {}", e));
    }

    if let Err(e) = replace_all_regex_in_file(
        &hive_file,
        &Regex::new(r"class .*\(PythonHivemind\)").unwrap(),
        format!("class {}Hivemind(PythonHivemind)", &hive_name),
    ) {
        return Err(format!("Failed to replace class name in hive.py: {}", e));
    }

    let config_file = config_file.to_string_lossy();

    BOT_FOLDER_SETTINGS.lock().unwrap().as_mut().unwrap().add_file(window, config_file.to_string());

    if open::that(hive_file).is_err() {
        ccprintln(
            window,
            format!("You have no default program to open .py files. Your new bot is located at {}", top_dir.to_string_lossy()),
        );
    }

    Ok(config_file.to_string())
}

/// Download and setup a new Rust bot
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `bot_name`: The name of the bot
/// * `directory`: The base directory to put the bot it, which must exist already
pub async fn bootstrap_rust_bot(window: &Window, bot_name: String, directory: PathBuf) -> Result<String, String> {
    let sanitized_name = sanitize(&bot_name);
    let top_dir = directory.join(&sanitized_name);

    if top_dir.exists() {
        return Err(format!("There is already a bot named {}, please choose a different name!", &sanitized_name));
    }

    download_extract_bot_template(window, "https://github.com/NicEastvillage/RLBotRustTemplateBot/archive/master.zip", top_dir.as_path()).await?;

    let config_file = top_dir.join("rustbot_dev").join("rustbot.cfg");

    let mut conf = load_cfg(&config_file)?;

    conf.set(BOT_CONFIG_MODULE_HEADER, NAME_KEY, Some(bot_name.clone()));
    conf.set(BOT_CONFIG_PARAMS_HEADER, EXECUTABLE_PATH_KEY, Some(format!("../target/debug/{}.exe", bot_name)));

    save_cfg(conf, &config_file)?;

    let cargo_toml_file = top_dir.join("Cargo.toml");

    let mut conf = load_cfg(&cargo_toml_file)?;

    conf.set("package", "name", Some(format!("\"{}\"", sanitized_name)));
    conf.set("package", "authors", Some("[\"\"]".to_owned()));

    save_cfg(conf, cargo_toml_file)?;

    if open::that(top_dir.join("src").join("main.rs")).is_err() {
        ccprintln(
            window,
            format!("You have no default program to open .rs files. Your new bot is located at {}", top_dir.to_string_lossy()),
        );
    }

    Ok(config_file.to_string_lossy().to_string())
}

/// Download and setup a new Scratch bot
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `bot_name`: The name of the bot
/// * `directory`: The base directory to put the bot it, which must exist already
pub async fn bootstrap_scratch_bot(window: &Window, bot_name: String, directory: PathBuf) -> Result<String, String> {
    let sanitized_name = sanitize(&bot_name);
    let top_dir = directory.join(&sanitized_name);

    if top_dir.exists() {
        return Err(format!("There is already a bot named {}, please choose a different name!", sanitized_name));
    }

    download_extract_bot_template(window, "https://github.com/RLBot/RLBotScratchInterface/archive/gui-friendly.zip", top_dir.as_path()).await?;

    // Choose appropriate file names based on the bot name
    let code_dir = top_dir.join(&sanitized_name);
    let sb3_filename = format!("{}.sb3", &sanitized_name);
    let sb3_file = code_dir.join(&sb3_filename);
    let config_filename = format!("{}.cfg", &sanitized_name);
    let config_file = code_dir.join(&config_filename);

    if let Err(e) = replace_all_regex_in_file(
        &top_dir.join("rlbot.cfg"),
        &Regex::new(r"(?P<a>participant_config_\d = ).*$").unwrap(),
        Regex::new(&format!(r"$a{}", Path::new(&sanitized_name).join(config_filename).to_string_lossy().replace('\\', "\\\\")))
            .unwrap()
            .to_string(),
    ) {
        return Err(format!("Failed to replace config file: {}", e));
    }

    // We're assuming that the file structure / names in RLBotScratchInterface will not change.
    // Semi-safe assumption because we're looking at a gui-specific git branch which ought to be stable.
    let copy_options = CopyOptions {
        copy_inside: true,
        ..Default::default()
    };

    if let Err(e) = move_dir(top_dir.join("scratch_bot"), &code_dir, &copy_options) {
        ccprintlne(window, e.to_string());
        return Err(format!("Failed to move scratch bot: {}", e));
    }

    if let Err(e) = rename(code_dir.join("my_scratch_bot.sb3"), sb3_file) {
        return Err(format!("Failed to rename scratch bot: {}", e));
    }

    let old_config_file = code_dir.join("my_scratch_bot.cfg");
    let mut conf = load_cfg(&old_config_file)?;

    conf.set(BOT_CONFIG_MODULE_HEADER, NAME_KEY, Some(bot_name.clone()));
    conf.set(BOT_CONFIG_PARAMS_HEADER, "sb3file", Some(sb3_filename));
    let random_port = rand::thread_rng().gen_range(20000..65000);
    conf.set(BOT_CONFIG_PARAMS_HEADER, "port", Some(random_port.to_string()));

    save_cfg(conf, &config_file)?;

    // delete the old config file
    if let Err(e) = remove_file(old_config_file) {
        return Err(format!("Failed to delete old config file: {}", e));
    }

    ccprintln(window, format!("Your new bot is located at {}", top_dir.to_string_lossy()));

    Ok(config_file.to_string_lossy().to_string())
}
