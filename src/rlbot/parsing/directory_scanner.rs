use glob::glob;
use std::{collections::HashSet, path::Path};

use super::bot_config_bundle::{BotConfigBundle, ScriptConfigBundle};

pub fn scan_directory_for_bot_configs(root_dir: &str) -> HashSet<BotConfigBundle> {
    let mut configs = HashSet::new();

    let pattern = Path::new(root_dir).join("**/*.cfg");

    for path in glob(pattern.to_str().unwrap()).unwrap().flatten() {
        if let Ok(bundle) = BotConfigBundle::from_path(path) {
            if bundle.is_valid_bot_config() {
                configs.insert(bundle);
            }
        }
    }

    configs
}

pub fn scan_directory_for_script_configs(root_dir: &str) -> HashSet<ScriptConfigBundle> {
    let mut configs = HashSet::new();

    let pattern = Path::new(root_dir).join("**/*.cfg");

    for path in glob(pattern.to_str().unwrap()).unwrap().flatten() {
        if let Ok(bundle) = ScriptConfigBundle::from_path(path) {
            if bundle.is_valid_script_config() {
                configs.insert(bundle);
            }
        }
    }

    configs
}