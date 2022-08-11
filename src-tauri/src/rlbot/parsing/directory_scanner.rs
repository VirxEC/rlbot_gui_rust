use super::bot_config_bundle::{BotConfigBundle, ScriptConfigBundle};
use futures_util::future::join_all;
use glob::glob;
use std::{collections::HashSet, path::Path};

/// Scan `root_dir` for BOTS (no scripts) and parse the configuration files, returning unique `BotConfigBundle`s
///
/// Does not load logos or missing python packages, but the paths to the logo file and requirements.txt WILL be loaded, if they exists
pub async fn scan_directory_for_bot_configs(root_dir: &str) -> HashSet<BotConfigBundle> {
    let pattern = Path::new(root_dir).join("**/*.cfg").to_string_lossy().to_string();
    let paths = glob(&pattern).unwrap().flatten().collect::<Vec<_>>();
    let configs = join_all(paths.iter().map(|path| BotConfigBundle::minimal_from_path(path.as_path())).collect::<Vec<_>>()).await;

    HashSet::from_iter(configs.into_iter().flatten())
}

/// Scan `root_dir` for SCRIPTS (no bots) and parse the configuration files, returning unique `ScriptConfigBundle`s
///
/// Does not load logos or missing python packages, but the paths to the logo file and requirements.txt WILL be loaded, if they exists
pub fn scan_directory_for_script_configs(root_dir: &str) -> HashSet<ScriptConfigBundle> {
    let pattern = Path::new(root_dir).join("**/*.cfg").to_string_lossy().to_string();
    let paths = glob(&pattern).unwrap().flatten().collect::<Vec<_>>();

    HashSet::from_iter(paths.iter().filter_map(|path| ScriptConfigBundle::minimal_from_path(path.as_path()).ok()))
}
