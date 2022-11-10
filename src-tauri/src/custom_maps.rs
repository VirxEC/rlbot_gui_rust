use crate::{rlbot::parsing::match_settings_config_parser::MapType, settings::BotFolder};
use glob::glob;
use std::collections::HashMap;

fn get_search_folders(bf: &HashMap<String, BotFolder>) -> Vec<String> {
    bf.iter().filter(|(_, bf)| bf.visible).map(|(path, _)| path.clone()).collect()
}

pub fn find_all(bf: &HashMap<String, BotFolder>) -> Vec<MapType> {
    get_search_folders(bf)
        .iter()
        .flat_map(|folder| {
            glob(&format!("{folder}/**/*.u[pd]k")).unwrap().flatten().filter_map(|match_| {
                let basename = match_.file_name()?.to_string_lossy();
                if basename.starts_with('_') {
                    None
                } else {
                    Some(MapType::Custom(basename.to_string()))
                }
            })
        })
        .collect()
}

pub fn convert_to_path(map: &str, bf: &HashMap<String, BotFolder>) -> Option<String> {
    get_search_folders(bf)
        .into_iter()
        .flat_map(|folder| glob(&format!("{folder}/**/{map}")))
        .flatten()
        .flatten()
        .next()
        .map(|pattern| pattern.to_string_lossy().to_string())
}
