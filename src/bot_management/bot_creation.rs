use std::collections::hash_map::DefaultHasher;
use std::fs::{write, File};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::{io::Cursor, path::Path};

use configparser::ini::Ini;
use regex::Regex;
use sanitize_filename::sanitize;

use crate::rlbot::parsing::bot_config_bundle::{BOT_CONFIG_MODULE_HEADER, NAME_KEY};
use crate::rlbot::parsing::directory_scanner::scan_directory_for_bot_configs;
use crate::{ccprintln, BOT_FOLDER_SETTINGS};

pub const CREATED_BOTS_FOLDER: &str = "MyBots";

pub async fn bootstrap_python_bot(bot_name: String, directory: &str) -> Result<String, String> {
    let sanitized_name = sanitize(&bot_name);
    let top_dir = Path::new(directory).join(CREATED_BOTS_FOLDER).join(&sanitized_name);

    if top_dir.exists() {
        return Err(format!("There is already a bot named {}, please choose a different name!", sanitized_name));
    }

    match reqwest::get("https://github.com/RLBot/RLBotPythonExample/archive/master.zip").await {
        Ok(res) => {
            zip_extract::extract(Cursor::new(&res.bytes().await.unwrap()), top_dir.as_path(), true).unwrap();
        }
        Err(e) => {
            return Err(format!("Failed to download python bot: {}", e));
        }
    }

    let bundles = scan_directory_for_bot_configs(top_dir.to_str().unwrap(), false);
    let bundle = bundles.iter().next().unwrap();
    let config_file = bundle.path.clone().unwrap();
    let python_file = bundle.python_path.clone().unwrap();

    let mut config = Ini::new();
    config.load(&config_file).unwrap();
    config.set(BOT_CONFIG_MODULE_HEADER, NAME_KEY, Some(bot_name));
    config.write(&config_file).unwrap();

    BOT_FOLDER_SETTINGS.lock().unwrap().add_file(config_file.clone());

    if open::that(python_file).is_err() {
        ccprintln(format!(
            "You have no default program to open .py files. Your new bot is located at {}",
            top_dir.to_str().unwrap()
        ));
    }

    Ok(config_file)
}

fn replace_all_regex_in_file(file_path: &Path, regex: &Regex, replacement: String) {
    let mut file = File::open(file_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let new_contents = regex.replace_all(&contents, replacement);
    write(file_path, new_contents.as_bytes()).unwrap();
}

pub async fn bootstrap_python_hivemind(hive_name: String, directory: &str) -> Result<String, String> {
    let sanitized_name = sanitize(&hive_name);
    let top_dir = Path::new(directory).join(CREATED_BOTS_FOLDER).join(&sanitized_name);

    if top_dir.exists() {
        return Err(format!("There is already a bot named {}, please choose a different name!", sanitized_name));
    }

    match reqwest::get("https://github.com/RLBot/RLBotPythonHivemindExample/archive/master.zip").await {
        Ok(res) => {
            zip_extract::extract(Cursor::new(&res.bytes().await.unwrap()), top_dir.as_path(), true).unwrap();
        }
        Err(e) => {
            return Err(format!("Failed to download python hivemind: {}", e));
        }
    }

    let config_file = top_dir.join("config.cfg");
    let drone_file = top_dir.join("src").join("drone.py");
    let hive_file = top_dir.join("src").join("hive.py");

    let mut config = Ini::new();
    config.load(&config_file).unwrap();
    config.set(BOT_CONFIG_MODULE_HEADER, NAME_KEY, Some(hive_name.clone()));
    config.write(&config_file).unwrap();

    replace_all_regex_in_file(&drone_file, &Regex::new(r"hive_name = .*$").unwrap(), format!("hive_name = \"{} Hivemind\"", &hive_name));

    let mut hasher = DefaultHasher::new();
    hive_name.hash(&mut hasher);
    let mut hive_key = hasher.finish();
    // add random number between 100000 and 999999 to hive_id
    hive_key += rand::random::<u64>() % 1000000;
    replace_all_regex_in_file(&drone_file, &Regex::new(r"hive_key = .*$").unwrap(), format!("hive_key = \"{}\"", hive_key));

    replace_all_regex_in_file(
        &hive_file,
        &Regex::new(r"class .*\(PythonHivemind\)").unwrap(),
        format!("class {}Hivemind(PythonHivemind)", &hive_name),
    );

    let config_file = config_file.to_str().unwrap();

    BOT_FOLDER_SETTINGS.lock().unwrap().add_file(config_file.to_string());

    if open::that(hive_file).is_err() {
        ccprintln(format!(
            "You have no default program to open .py files. Your new bot is located at {}",
            top_dir.to_str().unwrap()
        ));
    }

    Ok(config_file.to_string())
}
