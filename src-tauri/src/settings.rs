use crate::{
    ccprintlne,
    config_handles::load_gui_config,
    configparser::Ini,
    custom_maps::convert_to_path,
    get_config_path,
    rlbot::parsing::{
        bot_config_bundle::{Clean, ScriptConfigBundle},
        match_settings_config_parser::*,
    },
};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, str::FromStr};
use tauri::Window;

use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotFolder {
    pub visible: bool,
}

impl fmt::Display for BotFolder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).expect("Failed to serialize BotFolder"))
    }
}

impl FromStr for BotFolder {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotFolders {
    pub files: HashMap<String, BotFolder>,
    pub folders: HashMap<String, BotFolder>,
}

impl BotFolders {
    pub fn load_from_conf(conf: &Ini) -> Self {
        let files = serde_json::from_str(&conf.get("bot_folder_settings", "files").unwrap_or_else(|| String::from("[]"))).unwrap_or_default();

        let folders = serde_json::from_str(&*conf.get("bot_folder_settings", "folders").unwrap_or_else(|| String::from("[]"))).unwrap_or_default();

        Self { files, folders }
    }

    pub fn update_config(&mut self, window: &Window, bfs: Self) {
        *self = bfs;

        let mut conf = load_gui_config(window);
        conf.set("bot_folder_settings", "files", serde_json::to_string(&self.files).ok());
        conf.set("bot_folder_settings", "folders", serde_json::to_string(&self.folders).ok());

        if let Err(e) = conf.write(&get_config_path()) {
            ccprintlne(window, format!("Failed to write config file: {e}"));
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

fn set_value_in_conf<T: Default + serde::Serialize>(conf: &mut Ini, section: &str, key: &str, item: &T) {
    conf.set(section, key, serde_json::to_string(item).ok());
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MutatorConfig {
    pub match_length: MatchLength,
    pub max_score: MaxScore,
    pub overtime: Overtime,
    pub series_length: SeriesLength,
    pub game_speed: GameSpeed,
    pub ball_max_speed: BallMaxSpeed,
    pub ball_type: BallType,
    pub ball_weight: BallWeight,
    pub ball_size: BallSize,
    pub ball_bounciness: BallBounciness,
    pub boost_amount: BoostAmount,
    pub rumble: Rumble,
    pub boost_strength: BoostStrength,
    pub gravity: Gravity,
    pub demolish: Demolish,
    pub respawn_time: RespawnTime,
}

impl MutatorConfig {
    pub fn load(window: &Window) -> Self {
        let conf = load_gui_config(window);

        let match_length = conf.get("mutator_settings", "match_length").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let max_score = conf.get("mutator_settings", "max_score").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let overtime = conf.get("mutator_settings", "overtime").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let series_length = conf
            .get("mutator_settings", "series_length")
            .and_then(|x| serde_json::from_str(&x).ok())
            .unwrap_or_default();
        let game_speed = conf.get("mutator_settings", "game_speed").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let ball_max_speed = conf
            .get("mutator_settings", "ball_max_speed")
            .and_then(|x| serde_json::from_str(&x).ok())
            .unwrap_or_default();
        let ball_type = conf.get("mutator_settings", "ball_type").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let ball_weight = conf.get("mutator_settings", "ball_weight").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let ball_size = conf.get("mutator_settings", "ball_size").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let ball_bounciness = conf
            .get("mutator_settings", "ball_bounciness")
            .and_then(|x| serde_json::from_str(&x).ok())
            .unwrap_or_default();
        let boost_amount = conf.get("mutator_settings", "boost_amount").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let rumble = conf.get("mutator_settings", "rumble").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let boost_strength = conf
            .get("mutator_settings", "boost_strength")
            .and_then(|x| serde_json::from_str(&x).ok())
            .unwrap_or_default();
        let gravity = conf.get("mutator_settings", "gravity").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let demolish = conf.get("mutator_settings", "demolish").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let respawn_time = conf.get("mutator_settings", "respawn_time").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();

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

    fn set_value_in_conf<T: Default + serde::Serialize>(conf: &mut Ini, key: &str, item: &T) {
        set_value_in_conf(conf, "mutator_settings", key, item);
    }

    pub fn save_config(&self, conf: &mut Ini) {
        Self::set_value_in_conf(conf, "match_length", &self.match_length);
        Self::set_value_in_conf(conf, "max_score", &self.max_score);
        Self::set_value_in_conf(conf, "overtime", &self.overtime);
        Self::set_value_in_conf(conf, "series_length", &self.series_length);
        Self::set_value_in_conf(conf, "game_speed", &self.game_speed);
        Self::set_value_in_conf(conf, "ball_max_speed", &self.ball_max_speed);
        Self::set_value_in_conf(conf, "ball_type", &self.ball_type);
        Self::set_value_in_conf(conf, "ball_weight", &self.ball_weight);
        Self::set_value_in_conf(conf, "ball_size", &self.ball_size);
        Self::set_value_in_conf(conf, "ball_bounciness", &self.ball_bounciness);
        Self::set_value_in_conf(conf, "boost_amount", &self.boost_amount);
        Self::set_value_in_conf(conf, "rumble", &self.rumble);
        Self::set_value_in_conf(conf, "boost_strength", &self.boost_strength);
        Self::set_value_in_conf(conf, "gravity", &self.gravity);
        Self::set_value_in_conf(conf, "demolish", &self.demolish);
        Self::set_value_in_conf(conf, "respawn_time", &self.respawn_time);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MiniScriptBundle {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniMatchConfig {
    pub map: MapType,
    pub game_mode: GameMode,
    pub match_behavior: ExistingMatchBehavior,
    pub skip_replays: bool,
    pub instant_start: bool,
    pub enable_lockstep: bool,
    pub randomize_map: bool,
    pub enable_rendering: bool,
    pub enable_state_setting: bool,
    pub auto_save_replay: bool,
    pub scripts: Vec<MiniScriptBundle>,
    pub mutators: MutatorConfig,
}

impl Default for MiniMatchConfig {
    fn default() -> Self {
        Self {
            map: MapType::default(),
            game_mode: GameMode::default(),
            match_behavior: ExistingMatchBehavior::default(),
            skip_replays: false,
            instant_start: false,
            enable_lockstep: false,
            randomize_map: false,
            enable_rendering: false,
            enable_state_setting: true,
            auto_save_replay: false,
            scripts: Vec::new(),
            mutators: MutatorConfig::default(),
        }
    }
}

impl MiniMatchConfig {
    pub fn setup_for_start_match(&self, window: &Window, bf: &HashMap<String, BotFolder>) -> Result<Self, String> {
        let mut new = self.clone();

        if let MapType::Custom(path) = &mut new.map {
            *path = convert_to_path(path, bf).ok_or_else(|| {
                let err = format!("Failed to find custom map {path}");
                ccprintlne(window, err.clone());
                err
            })?;
        }

        Ok(new)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchConfig {
    pub map: MapType,
    pub game_mode: GameMode,
    pub match_behavior: ExistingMatchBehavior,
    pub skip_replays: bool,
    pub instant_start: bool,
    pub enable_lockstep: bool,
    pub randomize_map: bool,
    pub enable_rendering: bool,
    pub enable_state_setting: bool,
    pub auto_save_replay: bool,
    pub scripts: Vec<ScriptConfigBundle>,
    pub mutators: MutatorConfig,
}

impl MatchConfig {
    pub fn load(window: &Window) -> Self {
        let conf = load_gui_config(window);

        let map = conf.get("match_settings", "map").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let game_mode = conf.get("match_settings", "game_mode").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let match_behavior = conf.get("match_settings", "match_behavior").and_then(|x| serde_json::from_str(&x).ok()).unwrap_or_default();
        let skip_replays = conf.getbool("match_settings", "skip_replays").ok().flatten().unwrap_or_default();
        let instant_start = conf.getbool("match_settings", "instant_start").ok().flatten().unwrap_or_default();
        let enable_lockstep = conf.getbool("match_settings", "enable_lockstep").ok().flatten().unwrap_or_default();
        let randomize_map = conf.getbool("match_settings", "randomize_map").ok().flatten().unwrap_or_default();
        let enable_rendering = conf.getbool("match_settings", "enable_rendering").ok().flatten().unwrap_or_default();
        let enable_state_setting = conf.getbool("match_settings", "enable_state_setting").ok().flatten().unwrap_or(true);
        let auto_save_replay = conf.getbool("match_settings", "auto_save_replay").ok().flatten().unwrap_or_default();
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
            mutators: MutatorConfig::load(window),
        }
    }

    fn set_value_in_conf<T: Default + serde::Serialize>(conf: &mut Ini, key: &str, item: &T) {
        set_value_in_conf(conf, "match_settings", key, item);
    }

    pub fn save_to_config(&mut self, conf: &mut Ini) {
        Self::set_value_in_conf(conf, "map", &self.map);
        Self::set_value_in_conf(conf, "game_mode", &self.game_mode);
        Self::set_value_in_conf(conf, "match_behavior", &self.match_behavior);
        Self::set_value_in_conf(conf, "skip_replays", &self.skip_replays);
        Self::set_value_in_conf(conf, "instant_start", &self.instant_start);
        Self::set_value_in_conf(conf, "enable_lockstep", &self.enable_lockstep);
        Self::set_value_in_conf(conf, "randomize_map", &self.randomize_map);
        Self::set_value_in_conf(conf, "enable_rendering", &self.enable_rendering);
        Self::set_value_in_conf(conf, "enable_state_setting", &self.enable_state_setting);
        Self::set_value_in_conf(conf, "auto_save_replay", &self.auto_save_replay);
        Self::set_value_in_conf(conf, "scripts", &self.scripts);
        self.mutators.save_config(conf);
    }

    pub fn save_config(&mut self, window: &Window) {
        let mut conf = load_gui_config(window);
        self.save_to_config(&mut conf);

        if let Err(e) = conf.write(get_config_path()) {
            ccprintlne(window, format!("Error writing config file: {e}"));
        }
    }

    pub fn cleaned_scripts(&self) -> Self {
        let mut new = self.clone();
        new.scripts = clean(&new.scripts);
        new
    }
}

pub fn clean<T: Clean>(items: &[T]) -> Vec<T> {
    items.iter().map(Clean::cleaned).collect()
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

impl PackageResult {
    pub const fn new(exit_code: i32, packages: Vec<String>) -> Self {
        Self { exit_code, packages }
    }
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
pub struct LauncherConfig {
    pub preferred_launcher: String,
    pub use_login_tricks: bool,
    pub rocket_league_exe_path: Option<String>,
}

impl LauncherConfig {
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
            ccprintlne(window, format!("Error writing config file: {e}"));
        }
    }
}

#[derive(Serialize_repr, Deserialize_repr, Clone, Debug)]
#[repr(u8)]
pub enum Team {
    Blue,
    Orange,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TeamBotBundle {
    pub name: String,
    pub team: Team,
    pub skill: f32,
    pub runnable_type: String,
    pub path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Vec3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Rotation {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Physics {
    pub location: Vec3D,
    pub velocity: Vec3D,
    pub angular_velocity: Vec3D,
    pub rotation: Rotation,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Ball {
    pub physics: Physics,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Car {
    pub team: u8,
    pub physics: Physics,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct GameInfo {
    pub seconds_elapsed: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameTickPacket {
    pub game_ball: Ball,
    pub game_cars: Vec<Car>,
    pub game_info: GameInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoryTeamConfig {
    pub name: String,
    pub color: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub struct CustomConfig {
    pub story_path: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StoryIDs {
    Easy,
    Default,
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
pub struct StoryConfig {
    pub story_id: StoryIDs,
    pub use_custom_maps: bool,
    pub custom_config: CustomConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScoreResult {
    team_index: Team,
    score: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerStats {
    name: String,
    team: Team,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameResults {
    human_team: Team,
    score: Vec<ScoreResult>,
    stats: Vec<PlayerStats>,
    human_won: bool,
    timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChallengeAttempt {
    game_results: Option<GameResults>,
    challenge_completed: bool,
}

/// Represents users game state in RLBot Story Mode
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoryState {
    version: u8,
    story_settings: StoryConfig,
    team_settings: StoryTeamConfig,
    teammates: Vec<String>,
    challenges_attempts: HashMap<String, Vec<ChallengeAttempt>>,
    challenges_completed: HashMap<String, usize>,
    upgrades: HashMap<String, usize>,
}

impl StoryState {
    const CURRENCY_KEY: &'static str = "currency";

    pub fn new(team_settings: StoryTeamConfig, story_settings: StoryConfig) -> Self {
        Self {
            version: 1,
            story_settings,
            team_settings,
            teammates: Vec::new(),
            challenges_attempts: HashMap::new(),
            challenges_completed: HashMap::new(),
            upgrades: HashMap::from([(Self::CURRENCY_KEY.to_owned(), 0)]),
        }
    }

    pub fn save(&self, window: &Window) {
        let mut conf = load_gui_config(window);
        conf.set("story_mode", "save_state", serde_json::to_string(self).ok());

        if let Err(e) = conf.write(get_config_path()) {
            ccprintlne(window, format!("Failed to write config: {e}"));
        }
    }

    pub fn add_purchase(&mut self, id: String, cost: usize) -> Result<(), String> {
        let current_currency = *self.upgrades.get(Self::CURRENCY_KEY).ok_or("The key 'currency' was not found")?;
        if current_currency < cost {
            return Err(format!("Not enough currency to purchase {id}"));
        }

        if self.upgrades.contains_key(&id) {
            return Err(format!("Purchase already made: {id}"));
        }

        self.upgrades.insert(id, 1);
        self.upgrades.insert(Self::CURRENCY_KEY.to_owned(), current_currency - cost);

        Ok(())
    }

    pub fn add_recruit(&mut self, id: String) -> Result<(), String> {
        let current_currency = *self.upgrades.get(Self::CURRENCY_KEY).ok_or("The key 'currency' was not found")?;
        if current_currency < 1 {
            return Err(format!("Not enough currency to recruit {id}"));
        }

        self.teammates.push(id);
        self.upgrades.insert(Self::CURRENCY_KEY.to_owned(), current_currency - 1);

        Ok(())
    }

    pub const fn get_story_settings(&self) -> &StoryConfig {
        &self.story_settings
    }

    pub const fn get_upgrades(&self) -> &HashMap<String, usize> {
        &self.upgrades
    }

    pub const fn get_team_settings(&self) -> &StoryTeamConfig {
        &self.team_settings
    }
}
