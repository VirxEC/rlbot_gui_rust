// pub const MUTATOR_CONFIGURATION_HEADER: &str = "Mutator Configuration";
// pub const MUTATOR_MATCH_LENGTH: &str = "Match Length";
// pub const MUTATOR_MAX_SCORE: &str = "Max Score";
// pub const MUTATOR_OVERTIME: &str = "Overtime";
// pub const MUTATOR_SERIES_LENGTH: &str = "Series Length";
// pub const MUTATOR_GAME_SPEED: &str = "Game Speed";
// pub const MUTATOR_BALL_MAX_SPEED: &str = "Ball Max Speed";
// pub const MUTATOR_BALL_TYPE: &str = "Ball Type";
// pub const MUTATOR_BALL_WEIGHT: &str = "Ball Weight";
// pub const MUTATOR_BALL_SIZE: &str = "Ball Size";
// pub const MUTATOR_BALL_BOUNCINESS: &str = "Ball Bounciness";
// pub const MUTATOR_BOOST_AMOUNT: &str = "Boost Amount";
// pub const MUTATOR_RUMBLE: &str = "Rumble";
// pub const MUTATOR_BOOST_STRENGTH: &str = "Boost Strength";
// pub const MUTATOR_GRAVITY: &str = "Gravity";
// pub const MUTATOR_DEMOLISH: &str = "Demolish";
// pub const MUTATOR_RESPAWN_TIME: &str = "Respawn Time";

// pub const MATCH_CONFIGURATION_HEADER: &str = "Match Configuration";
// pub const PARTICIPANT_COUNT_KEY: &str = "num_participants";
// pub const GAME_MODE: &str = "game_mode";
// pub const GAME_MAP: &str = "game_map";
// pub const SKIP_REPLAYS: &str = "skip_replays";
// pub const INSTANT_START: &str = "start_without_countdown";
// pub const EXISTING_MATCH_BEHAVIOR: &str = "existing_match_behavior";
// pub const ENABLE_LOCKSTEP: &str = "enable_lockstep";
// pub const ENABLE_RENDERING: &str = "enable_rendering";
// pub const ENABLE_STATE_SETTING: &str = "enable_state_setting";
// pub const AUTO_SAVE_REPLAY: &str = "auto_save_replay";

pub const MAP_TYPES: [&str; 60] = [
    "DFHStadium",
    "Mannfield",
    "ChampionsField",
    "UrbanCentral",
    "BeckwithPark",
    "UtopiaColiseum",
    "Wasteland",
    "NeoTokyo",
    "AquaDome",
    "StarbaseArc",
    "Farmstead",
    "SaltyShores",
    "DFHStadium_Stormy",
    "DFHStadium_Day",
    "Mannfield_Stormy",
    "Mannfield_Night",
    "ChampionsField_Day",
    "BeckwithPark_Stormy",
    "BeckwithPark_Midnight",
    "UrbanCentral_Night",
    "UrbanCentral_Dawn",
    "UtopiaColiseum_Dusk",
    "DFHStadium_Snowy",
    "Mannfield_Snowy",
    "UtopiaColiseum_Snowy",
    "Badlands",
    "Badlands_Night",
    "TokyoUnderpass",
    "Arctagon",
    "Pillars",
    "Cosmic",
    "DoubleGoal",
    "Octagon",
    "Underpass",
    "UtopiaRetro",
    "Hoops_DunkHouse",
    "DropShot_Core707",
    "ThrowbackStadium",
    "ForbiddenTemple",
    "RivalsArena",
    "Farmstead_Night",
    "SaltyShores_Night",
    "NeonFields",
    "DFHStadium_Circuit",
    "DeadeyeCanyon",
    "StarbaseArc_Aftermath",
    "Wasteland_Night",
    "BeckwithPark_GothamNight",
    "ForbiddenTemple_Day",
    "UrbanCentral_Haunted",
    "ChampionsField_NFL",
    "ThrowbackStadium_Snowy",
    "Basin",
    "Corridor",
    "Loophole",
    "Galleon",
    "GalleonRetro",
    "Hourglass",
    "Barricade",
    "Colossus",
];

pub const GAME_MODES: [&str; 7] = ["Soccer", "Hoops", "Dropshot", "Hockey", "Rumble", "Heatseeker", "Gridiron"];
pub const EXISTING_MATCH_BEHAVIOR_TYPES: [&str; 3] = ["Restart If Different", "Restart", "Continue And Spawn"];

pub const MATCH_LENGTH_TYPES: [&str; 4] = ["5 Minutes", "10 Minutes", "20 Minutes", "Unlimited"];
pub const MAX_SCORE_TYPES: [&str; 4] = ["Unlimited", "1 Goal", "3 Goals", "5 Goals"];
pub const OVERTIME_MUTATOR_TYPES: [&str; 3] = ["Unlimited", "+5 Max, First Score", "+5 Max, Random Team"];
pub const SERIES_LENGTH_MUTATOR_TYPES: [&str; 4] = ["Unlimited", "3 Games", "5 Games", "7 Games"];
pub const GAME_SPEED_MUTATOR_TYPES: [&str; 3] = ["Default", "Slo-Mo", "Time Warp"];
pub const BALL_MAX_SPEED_MUTATOR_TYPES: [&str; 4] = ["Default", "Slow", "Fast", "Super Fast"];
pub const BALL_TYPE_MUTATOR_TYPES: [&str; 4] = ["Default", "Cube", "Puck", "Basketball"];
pub const BALL_WEIGHT_MUTATOR_TYPES: [&str; 4] = ["Default", "Light", "Heavy", "Super Light"];
pub const BALL_SIZE_MUTATOR_TYPES: [&str; 4] = ["Default", "Small", "Large", "Gigantic"];
pub const BALL_BOUNCINESS_MUTATOR_TYPES: [&str; 4] = ["Default", "Low", "High", "Super High"];
pub const BOOST_AMOUNT_MUTATOR_TYPES: [&str; 5] = ["Default", "Unlimited", "Recharge (Slow)", "Recharge (Fast)", "No Boost"];
pub const RUMBLE_MUTATOR_TYPES: [&str; 8] = ["None", "Default", "Slow", "Civilized", "Destruction Derby", "Spring Loaded", "Spikes Only", "Spike Rush"];
pub const BOOST_STRENGTH_MUTATOR_TYPES: [&str; 4] = ["1x", "1.5x", "2x", "10x"];
pub const GRAVITY_MUTATOR_TYPES: [&str; 4] = ["Default", "Low", "High", "Super High"];
pub const DEMOLISH_MUTATOR_TYPES: [&str; 5] = ["Default", "Disabled", "Friendly Fire", "On Contact", "On Contact (FF)"];
pub const RESPAWN_TIME_MUTATOR_TYPES: [&str; 4] = ["3 Seconds", "2 Seconds", "1 Second", "Disable Goal Reset"];

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Mutators {
    pub match_length_types: Vec<String>,
    pub max_score_types: Vec<String>,
    pub overtime_types: Vec<String>,
    pub series_length_types: Vec<String>,
    pub game_speed_types: Vec<String>,
    pub ball_max_speed_types: Vec<String>,
    pub ball_type_types: Vec<String>,
    pub ball_weight_types: Vec<String>,
    pub ball_size_types: Vec<String>,
    pub ball_bounciness_types: Vec<String>,
    pub boost_amount_types: Vec<String>,
    pub rumble_types: Vec<String>,
    pub boost_strength_types: Vec<String>,
    pub gravity_types: Vec<String>,
    pub demolish_types: Vec<String>,
    pub respawn_time_types: Vec<String>,
}

fn vec_to_string(vec: &[&str]) -> Vec<String> {
    vec.iter().map(|s| s.to_string()).collect()
}

impl Mutators {
    pub fn new() -> Self {
        Self {
            match_length_types: vec_to_string(&MATCH_LENGTH_TYPES),
            max_score_types: vec_to_string(&MAX_SCORE_TYPES),
            overtime_types: vec_to_string(&OVERTIME_MUTATOR_TYPES),
            series_length_types: vec_to_string(&SERIES_LENGTH_MUTATOR_TYPES),
            game_speed_types: vec_to_string(&GAME_SPEED_MUTATOR_TYPES),
            ball_max_speed_types: vec_to_string(&BALL_MAX_SPEED_MUTATOR_TYPES),
            ball_type_types: vec_to_string(&BALL_TYPE_MUTATOR_TYPES),
            ball_weight_types: vec_to_string(&BALL_WEIGHT_MUTATOR_TYPES),
            ball_size_types: vec_to_string(&BALL_SIZE_MUTATOR_TYPES),
            ball_bounciness_types: vec_to_string(&BALL_BOUNCINESS_MUTATOR_TYPES),
            boost_amount_types: vec_to_string(&BOOST_AMOUNT_MUTATOR_TYPES),
            rumble_types: vec_to_string(&RUMBLE_MUTATOR_TYPES),
            boost_strength_types: vec_to_string(&BOOST_STRENGTH_MUTATOR_TYPES),
            gravity_types: vec_to_string(&GRAVITY_MUTATOR_TYPES),
            demolish_types: vec_to_string(&DEMOLISH_MUTATOR_TYPES),
            respawn_time_types: vec_to_string(&RESPAWN_TIME_MUTATOR_TYPES),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MatchOptions {
    pub map_types: Vec<String>,
    pub game_modes: Vec<String>,
    pub match_behaviours: Vec<String>,
    pub mutators: Mutators,
}

impl MatchOptions {
    pub fn new() -> Self {
        Self {
            map_types: vec_to_string(&MAP_TYPES),
            game_modes: vec_to_string(&GAME_MODES),
            match_behaviours: vec_to_string(&EXISTING_MATCH_BEHAVIOR_TYPES),
            mutators: Mutators::new(),
        }
    }
}
