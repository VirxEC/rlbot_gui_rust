// pub const CUSTOM_MAP_TARGET: (&str, &str) = ("Labs_Utopia_P.upk", "UtopiaRetro");

use std::collections::HashMap;

use crate::BotFolder;

use glob::glob;

fn get_search_folders(bf: &HashMap<String, BotFolder>) -> Vec<String> {
    bf.iter().filter(|(_, bf)| bf.visible).map(|(path, _)| path.clone()).collect()
}

pub fn find_all_custom_maps(bf: &HashMap<String, BotFolder>) -> Vec<String> {
    get_search_folders(bf)
        .iter()
        .flat_map(|folder| {
            glob(&format!("{}/**/*.u[pd]k", folder)).unwrap().flatten().filter_map(|match_| {
                let basename = match_.file_name().unwrap().to_str().unwrap();
                if !basename.starts_with('_') {
                    Some(basename.to_string())
                } else {
                    None
                }
            })
        })
        .collect()
}
