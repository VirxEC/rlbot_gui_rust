use crate::rlbot::parsing::match_settings_config_parser::{MapType, MaxScore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod default;
pub mod easy;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct StoryModeConfig {
    pub settings: Settings,
    pub bots: HashMap<String, Bot>,
    pub cities: HashMap<String, City>,
    pub scripts: HashMap<String, Script>,
}


#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub min_map_pack_revision: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct City {
    pub description: Description,
    pub challenges: Vec<Challenge>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Description {
    pub message: String,
    pub prereqs: Vec<String>,
    pub color: Option<u16>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Challenge {
    pub id: String,
    pub human_team_size: u8,
    pub opponent_bots: Vec<String>,
    #[serde(rename = "max_score")]
    pub max_score: MaxScore,
    pub limitations: Vec<String>,
    pub map: MapType,
    pub disabled_boost: bool,
    pub display: String,
    pub completion_conditions: Option<CompletionConditions>,
    pub scripts: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CompletionConditions {
    pub win: Option<bool>,
    pub score_difference: Option<i16>,
    pub self_demo_count: Option<u16>,
    pub demo_achieved_count: Option<u16>,
    pub goals_scored: Option<i16>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BotType {
    #[default]
    Psyonix,
    RLBot,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Bot {
    pub name: String,
    #[serde(rename = "type")]
    pub type_field: BotType,
    pub skill: Option<f64>,
    pub path: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub path: Vec<String>,
}

#[macro_export]
macro_rules! storymode_json {
    ($($json:tt)+) => {
        use $crate::stories::cmaps::StoryModeConfig;
        use once_cell::sync::Lazy;
        use serde_json::{from_value, json};

        static JSON: Lazy<StoryModeConfig> = Lazy::new(|| match from_value(json!({ $($json)+ })) {
            Ok(json) => json,
            Err(err) => panic!("Failed to parse Story Mode JSON config: {}", err),
        });

        pub fn json() -> StoryModeConfig {
            JSON.clone()
        }
    };
}
