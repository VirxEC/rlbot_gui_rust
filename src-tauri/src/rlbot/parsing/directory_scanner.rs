use super::bot_config_bundle::{BotConfigBundle, ScriptConfigBundle};
use futures_util::future::join_all;
use glob::glob;
use std::path::Path;

/// Scan `root_dir` for BOTS (no scripts) and parse the configuration files, returning unique `BotConfigBundle`s
///
/// Does not load logos or missing python packages, but the paths to the logo file and requirements.txt WILL be loaded, if they exists
pub async fn scan_directory_for_bot_configs(root_dir: &str) -> Vec<BotConfigBundle> {
    let pattern = Path::new(root_dir).join("**/*.cfg").to_string_lossy().to_string();
    let paths = glob(&pattern).unwrap().flatten().collect::<Vec<_>>();

    join_all(paths.iter().map(BotConfigBundle::minimal_from_path)).await.into_iter().flatten().collect()
}

/// Scan `root_dir` for SCRIPTS (no bots) and parse the configuration files, returning unique `ScriptConfigBundle`s
///
/// Does not load logos or missing python packages, but the paths to the logo file and requirements.txt WILL be loaded, if they exists
pub async fn scan_directory_for_script_configs(root_dir: &str) -> Vec<ScriptConfigBundle> {
    let pattern = Path::new(root_dir).join("**/*.cfg").to_string_lossy().to_string();
    let paths = glob(&pattern).unwrap().flatten().collect::<Vec<_>>();

    join_all(paths.iter().map(ScriptConfigBundle::minimal_from_path)).await.into_iter().flatten().collect()
}
