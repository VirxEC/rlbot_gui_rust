use super::bot_config_bundle::{BotConfigBundle, RLBotCfgParseError, ScriptConfigBundle};
use crate::ccprintln;
use futures_util::{future::join_all, Future};
use glob::glob;
use std::path::PathBuf;
use tauri::Window;

/// Scan `root_dir` for BOTS (no scripts) and parse the configuration files, returning unique `BotConfigBundle`s
///
/// Does not load logos or missing python packages, but the paths to the logo file and requirements.txt WILL be loaded, if they exists
pub async fn scan_directory_for_bot_configs(window: &Window, root_dir: &str) -> Vec<BotConfigBundle> {
    scan_directory_for_item(window, root_dir, BotConfigBundle::minimal_from_path).await
}

/// Scan `root_dir` for SCRIPTS (no bots) and parse the configuration files, returning unique `ScriptConfigBundle`s
///
/// Does not load logos or missing python packages, but the paths to the logo file and requirements.txt WILL be loaded, if they exists
pub async fn scan_directory_for_script_configs(window: &Window, root_dir: &str) -> Vec<ScriptConfigBundle> {
    scan_directory_for_item(window, root_dir, ScriptConfigBundle::minimal_from_path).await
}

/// Scan `root_dir` for and run func on each item found, filtering items that returned errors.
/// func must be async and return a `Result<T, RLBotCfgParError>`.
/// func will be ran on all items found in the directory at the same time (via `join_all`).
async fn scan_directory_for_item<T, R, F>(window: &Window, root_dir: &str, func: F) -> Vec<T>
where
    T: Sized,
    R: Future<Output = Result<T, RLBotCfgParseError>>,
    F: FnMut(PathBuf) -> R,
{
    join_all(glob(&format!("{root_dir}/**/*.cfg")).unwrap().flatten().map(func))
        .await
        .into_iter()
        .filter_map(|bundle| match bundle {
            Ok(bundle) => Some(bundle),
            Err(err) => {
                if let RLBotCfgParseError::NoPythonFile(_) = err {
                    return None;
                }

                if let RLBotCfgParseError::NoScriptFile(_) = err {
                    return None;
                }

                ccprintln(window, err.to_string());

                None
            }
        })
        .collect()
}
