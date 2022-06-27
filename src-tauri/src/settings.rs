use crate::{
    ccprintlne,
    config_handles::load_gui_config,
    custom_maps::convert_custom_map_to_path,
    get_config_path,
    rlbot::parsing::{
        bot_config_bundle::{Clean, ScriptConfigBundle},
        match_settings_config_parser::*,
    },
};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use tauri::Window;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotFolder {
    pub visible: bool,
}

impl fmt::Display for BotFolder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl FromStr for BotFolder {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BotFolderSettings {
    pub files: HashMap<String, BotFolder>,
    pub folders: HashMap<String, BotFolder>,
}

impl BotFolderSettings {
    pub fn load(window: &Window) -> Self {
        let conf = load_gui_config(window);
        let files = serde_json::from_str(&conf.get("bot_folder_settings", "files").unwrap_or_else(|| String::from("[]"))).unwrap_or_default();

        let folders = serde_json::from_str(&*conf.get("bot_folder_settings", "folders").unwrap_or_else(|| String::from("[]"))).unwrap_or_default();

        Self { files, folders }
    }

    pub fn update_config(&mut self, window: &Window, bfs: Self) {
        self.files = bfs.files;
        self.folders = bfs.folders;

        let path = get_config_path();
        let mut conf = load_gui_config(window);
        conf.set("bot_folder_settings", "files", serde_json::to_string(&self.files).ok());
        conf.set("bot_folder_settings", "folders", serde_json::to_string(&self.folders).ok());

        if let Err(e) = conf.write(&path) {
            ccprintlne(window, format!("Failed to write config file: {}", e));
        }
    }

    pub fn add_folder(&mut self, window: &Window, path: String) {
        self.folders.insert(path, BotFolder { visible: true });
        self.update_config(window, self.clone());
    }

    pub fn add_file(&mut self, window: &Window, path: String) {
        self.files.insert(path, BotFolder { visible: true });
        self.update_config(window, self.clone());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MutatorSettings {
    pub match_length: String,
    pub max_score: String,
    pub overtime: String,
    pub series_length: String,
    pub game_speed: String,
    pub ball_max_speed: String,
    pub ball_type: String,
    pub ball_weight: String,
    pub ball_size: String,
    pub ball_bounciness: String,
    pub boost_amount: String,
    pub rumble: String,
    pub boost_strength: String,
    pub gravity: String,
    pub demolish: String,
    pub respawn_time: String,
}

impl MutatorSettings {
    pub fn load(window: &Window) -> Self {
        let conf = load_gui_config(window);

        let match_length = conf.get("mutator_settings", "match_length").unwrap_or_else(|| MATCH_LENGTH_TYPES[0].to_owned());
        let max_score = conf.get("mutator_settings", "max_score").unwrap_or_else(|| MAX_SCORE_TYPES[0].to_owned());
        let overtime = conf.get("mutator_settings", "overtime").unwrap_or_else(|| OVERTIME_MUTATOR_TYPES[0].to_owned());
        let series_length = conf.get("mutator_settings", "series_length").unwrap_or_else(|| SERIES_LENGTH_MUTATOR_TYPES[0].to_owned());
        let game_speed = conf.get("mutator_settings", "game_speed").unwrap_or_else(|| GAME_SPEED_MUTATOR_TYPES[0].to_owned());
        let ball_max_speed = conf.get("mutator_settings", "ball_max_speed").unwrap_or_else(|| BALL_MAX_SPEED_MUTATOR_TYPES[0].to_owned());
        let ball_type = conf.get("mutator_settings", "ball_type").unwrap_or_else(|| BALL_TYPE_MUTATOR_TYPES[0].to_owned());
        let ball_weight = conf.get("mutator_settings", "ball_weight").unwrap_or_else(|| BALL_WEIGHT_MUTATOR_TYPES[0].to_owned());
        let ball_size = conf.get("mutator_settings", "ball_size").unwrap_or_else(|| BALL_SIZE_MUTATOR_TYPES[0].to_owned());
        let ball_bounciness = conf
            .get("mutator_settings", "ball_bounciness")
            .unwrap_or_else(|| BALL_BOUNCINESS_MUTATOR_TYPES[0].to_owned());
        let boost_amount = conf.get("mutator_settings", "boost_amount").unwrap_or_else(|| BOOST_AMOUNT_MUTATOR_TYPES[0].to_owned());
        let rumble = conf.get("mutator_settings", "rumble").unwrap_or_else(|| RUMBLE_MUTATOR_TYPES[0].to_owned());
        let boost_strength = conf.get("mutator_settings", "boost_strength").unwrap_or_else(|| BOOST_STRENGTH_MUTATOR_TYPES[0].to_owned());
        let gravity = conf.get("mutator_settings", "gravity").unwrap_or_else(|| GRAVITY_MUTATOR_TYPES[0].to_owned());
        let demolish = conf.get("mutator_settings", "demolish").unwrap_or_else(|| DEMOLISH_MUTATOR_TYPES[0].to_owned());
        let respawn_time = conf.get("mutator_settings", "respawn_time").unwrap_or_else(|| RESPAWN_TIME_MUTATOR_TYPES[0].to_owned());

        Self {
            match_length,
            max_score,
            overtime,
            series_length,
            game_speed,
            ball_max_speed,
            ball_type,
            ball_weight,
            ball_size,
            ball_bounciness,
            boost_amount,
            rumble,
            boost_strength,
            gravity,
            demolish,
            respawn_time,
        }
    }

    pub fn update_config(&mut self, window: &Window, ms: Self) {
        self.match_length = ms.match_length;
        self.max_score = ms.max_score;
        self.overtime = ms.overtime;
        self.series_length = ms.series_length;
        self.game_speed = ms.game_speed;
        self.ball_max_speed = ms.ball_max_speed;
        self.ball_type = ms.ball_type;
        self.ball_weight = ms.ball_weight;
        self.ball_size = ms.ball_size;
        self.ball_bounciness = ms.ball_bounciness;
        self.boost_amount = ms.boost_amount;
        self.rumble = ms.rumble;
        self.boost_strength = ms.boost_strength;
        self.gravity = ms.gravity;
        self.demolish = ms.demolish;
        self.respawn_time = ms.respawn_time;

        let mut conf = load_gui_config(window);
        conf.set("mutator_settings", "match_length", Some(self.match_length.clone()));
        conf.set("mutator_settings", "max_score", Some(self.max_score.clone()));
        conf.set("mutator_settings", "overtime", Some(self.overtime.clone()));
        conf.set("mutator_settings", "series_length", Some(self.series_length.clone()));
        conf.set("mutator_settings", "game_speed", Some(self.game_speed.clone()));
        conf.set("mutator_settings", "ball_max_speed", Some(self.ball_max_speed.clone()));
        conf.set("mutator_settings", "ball_type", Some(self.ball_type.clone()));
        conf.set("mutator_settings", "ball_weight", Some(self.ball_weight.clone()));
        conf.set("mutator_settings", "ball_size", Some(self.ball_size.clone()));
        conf.set("mutator_settings", "ball_bounciness", Some(self.ball_bounciness.clone()));
        conf.set("mutator_settings", "boost_amount", Some(self.boost_amount.clone()));
        conf.set("mutator_settings", "rumble", Some(self.rumble.clone()));
        conf.set("mutator_settings", "boost_strength", Some(self.boost_strength.clone()));
        conf.set("mutator_settings", "gravity", Some(self.gravity.clone()));
        conf.set("mutator_settings", "demolish", Some(self.demolish.clone()));
        conf.set("mutator_settings", "respawn_time", Some(self.respawn_time.clone()));

        if let Err(e) = conf.write(get_config_path()) {
            ccprintlne(window, format!("Error writing config file: {}", e));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchSettings {
    pub map: String,
    pub game_mode: String,
    pub match_behavior: String,
    pub skip_replays: bool,
    pub instant_start: bool,
    pub enable_lockstep: bool,
    pub randomize_map: bool,
    pub enable_rendering: bool,
    pub enable_state_setting: bool,
    pub auto_save_replay: bool,
    pub scripts: Vec<ScriptConfigBundle>,
    pub mutators: MutatorSettings,
}

impl MatchSettings {
    pub fn load(window: &Window) -> Self {
        let conf = load_gui_config(window);

        let map = conf.get("match_settings", "map").unwrap_or_else(|| MAP_TYPES[0].to_owned());
        let game_mode = conf.get("match_settings", "game_mode").unwrap_or_else(|| GAME_MODES[0].to_owned());
        let match_behavior = conf.get("match_settings", "match_behavior").unwrap_or_else(|| EXISTING_MATCH_BEHAVIOR_TYPES[0].to_owned());
        let skip_replays = conf.getbool("match_settings", "skip_replays").unwrap_or(Some(false)).unwrap_or(false);
        let instant_start = conf.getbool("match_settings", "instant_start").unwrap_or(Some(false)).unwrap_or(false);
        let enable_lockstep = conf.getbool("match_settings", "enable_lockstep").unwrap_or(Some(false)).unwrap_or(false);
        let randomize_map = conf.getbool("match_settings", "randomize_map").unwrap_or(Some(false)).unwrap_or(false);
        let enable_rendering = conf.getbool("match_settings", "enable_rendering").unwrap_or(Some(false)).unwrap_or(false);
        let enable_state_setting = conf.getbool("match_settings", "enable_state_setting").unwrap_or(Some(true)).unwrap_or(true);
        let auto_save_replay = conf.getbool("match_settings", "auto_save_replay").unwrap_or(Some(false)).unwrap_or(false);
        let scripts = serde_json::from_str(&conf.get("match_settings", "scripts").unwrap_or_else(|| "[]".to_owned())).unwrap_or_default();

        Self {
            map,
            game_mode,
            match_behavior,
            skip_replays,
            instant_start,
            enable_lockstep,
            randomize_map,
            enable_rendering,
            enable_state_setting,
            auto_save_replay,
            scripts,
            mutators: MutatorSettings::load(window),
        }
    }

    pub fn update_config(&mut self, window: &Window, ms: Self) {
        self.map = ms.map;
        self.game_mode = ms.game_mode;
        self.match_behavior = ms.match_behavior;
        self.skip_replays = ms.skip_replays;
        self.instant_start = ms.instant_start;
        self.enable_lockstep = ms.enable_lockstep;
        self.randomize_map = ms.randomize_map;
        self.enable_rendering = ms.enable_rendering;
        self.enable_state_setting = ms.enable_state_setting;
        self.auto_save_replay = ms.auto_save_replay;
        self.scripts = ms.scripts;

        self.mutators.update_config(window, ms.mutators);

        let mut conf = load_gui_config(window);
        conf.set("match_settings", "map", Some(self.map.clone()));
        conf.set("match_settings", "game_mode", Some(self.game_mode.clone()));
        conf.set("match_settings", "match_behavior", Some(self.match_behavior.clone()));
        conf.set("match_settings", "skip_replays", Some(self.skip_replays.to_string()));
        conf.set("match_settings", "instant_start", Some(self.instant_start.to_string()));
        conf.set("match_settings", "enable_lockstep", Some(self.enable_lockstep.to_string()));
        conf.set("match_settings", "randomize_map", Some(self.randomize_map.to_string()));
        conf.set("match_settings", "enable_rendering", Some(self.enable_rendering.to_string()));
        conf.set("match_settings", "enable_state_setting", Some(self.enable_state_setting.to_string()));
        conf.set("match_settings", "auto_save_replay", Some(self.auto_save_replay.to_string()));
        conf.set("match_settings", "scripts", Some(serde_json::to_string(&self.scripts).unwrap_or_default()));

        if let Err(e) = conf.write(get_config_path()) {
            ccprintlne(window, format!("Error writing config file: {}", e));
        }
    }

    pub fn cleaned_scripts(&self) -> Self {
        let mut new = self.clone();
        new.scripts = clean(new.scripts);
        new
    }

    pub fn setup_for_start_match(&self, window: &Window, bf: &HashMap<String, BotFolder>) -> Option<Self> {
        let mut new = self.clone();

        for script in &mut new.scripts {
            script.info = None;
            *script = script.cleaned();
        }

        if new.map.ends_with(".upk") || new.map.ends_with(".udk") {
            new.map = match convert_custom_map_to_path(&new.map, bf) {
                Some(path) => path,
                None => {
                    ccprintlne(window, format!("Failed to find custom map {}", new.map));
                    return None;
                }
            };
        }

        Some(new)
    }
}

pub fn clean<T: Clean>(items: Vec<T>) -> Vec<T> {
    items.iter().map(|i| i.cleaned()).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recommendation<T> {
    bots: Vec<T>,
}

impl<T> Recommendation<T> {
    fn change_generic<F>(&self, f: &dyn Fn(&T) -> F) -> Recommendation<F> {
        Recommendation {
            bots: self.bots.iter().map(f).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllRecommendations<T> {
    recommendations: Vec<Recommendation<T>>,
}

impl<T> AllRecommendations<T> {
    pub fn change_generic<F>(&self, f: &dyn Fn(&T) -> F) -> AllRecommendations<F> {
        AllRecommendations {
            recommendations: self.recommendations.iter().map(|i| i.change_generic(f)).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackageResult {
    pub exit_code: i32,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingPackagesUpdate {
    pub index: usize,
    pub warn: Option<String>,
    pub missing_packages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoUpdate {
    pub index: usize,
    pub logo: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConsoleText {
    pub text: String,
    pub color: Option<String>,
}

impl ConsoleText {
    pub const fn from(text: String, color: Option<String>) -> ConsoleText {
        ConsoleText { text, color }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleTextUpdate {
    pub content: ConsoleText,
    pub replace_last: bool,
}

impl ConsoleTextUpdate {
    const fn new(text: String, color: Option<String>, replace_last: bool) -> Self {
        ConsoleTextUpdate {
            content: ConsoleText::from(text, color),
            replace_last,
        }
    }

    pub fn from(text: String, replace_last: bool) -> Self {
        let color = {
            let text = text.to_ascii_lowercase();
            if text.contains("error") {
                Some("red".to_owned())
            } else if text.contains("warning") {
                Some("#A1761B".to_owned())
            } else if text.contains("info") {
                Some("blue".to_owned())
            } else {
                None
            }
        };

        ConsoleTextUpdate::new(text, color, replace_last)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LauncherSettings {
    pub preferred_launcher: String,
    pub use_login_tricks: bool,
    pub rocket_league_exe_path: Option<String>,
}

impl LauncherSettings {
    pub fn load(window: &Window) -> Self {
        let config = load_gui_config(window);

        Self {
            preferred_launcher: config.get("launcher_settings", "preferred_launcher").unwrap_or_else(|| "epic".to_owned()),
            use_login_tricks: config.getbool("launcher_settings", "use_login_tricks").unwrap_or_default().unwrap_or(true),
            rocket_league_exe_path: config.get("launcher_settings", "rocket_league_exe_path"),
        }
    }

    pub fn write_to_file(self, window: &Window) {
        let mut config = load_gui_config(window);

        config.set("launcher_settings", "preferred_launcher", Some(self.preferred_launcher));
        config.set("launcher_settings", "use_login_tricks", Some(self.use_login_tricks.to_string()));
        config.set("launcher_settings", "rocket_league_exe_path", self.rocket_league_exe_path);

        if let Err(e) = config.write(get_config_path()) {
            ccprintlne(window, format!("Error writing config file: {}", e));
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TeamBotBundle {
    pub name: String,
    pub team: u8,
    pub skill: f32,
    pub runnable_type: String,
    pub path: Option<String>,
}
