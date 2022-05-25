use glob::glob;
use rayon::iter::{FromParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashSet, path::Path};

use super::bot_config_bundle::{BotConfigBundle, ScriptConfigBundle};

pub fn scan_directory_for_bot_configs(root_dir: &str, has_rlbot: bool) -> HashSet<BotConfigBundle> {
    let pattern = Path::new(root_dir).join("**/*.cfg");
    let paths = glob(pattern.to_str().unwrap()).unwrap().flatten().collect::<Vec<_>>();

    HashSet::from_par_iter(paths.par_iter().filter_map(|path| BotConfigBundle::from_path(path.as_path(), has_rlbot).ok()))
}

pub fn scan_directory_for_script_configs(root_dir: &str, has_rlbot: bool) -> HashSet<ScriptConfigBundle> {
    let pattern = Path::new(root_dir).join("**/*.cfg");
    let paths = glob(pattern.to_str().unwrap()).unwrap().flatten().collect::<Vec<_>>();

    HashSet::from_par_iter(paths.par_iter().filter_map(|path| ScriptConfigBundle::from_path(path.as_path(), has_rlbot).ok()))
}
