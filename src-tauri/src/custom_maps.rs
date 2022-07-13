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
            glob(&format!("{}/**/*.u[pd]k", folder)).unwrap().flatten().filter_map(|match_| {
                let basename = match_.file_name().unwrap().to_string_lossy();
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
    for folder in get_search_folders(bf) {
        if let Some(map_path) = glob(&format!("{}/**/{}", folder, map)).unwrap().flatten().next() {
            return Some(map_path.to_string_lossy().to_string());
        }
    }
    None
}
