use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Deserialize_enum_str, Serialize_enum_str, Clone, Debug, Default, EnumIter)]
pub enum MapType {
    #[default]
    #[serde(rename = "DFHStadium")]
    DfhStadium,
    Mannfield,
    ChampionsField,
    UrbanCentral,
    BeckwithPark,
    UtopiaColiseum,
    Wasteland,
    NeoTokyo,
    AquaDome,
    StarbaseArc,
    Farmstead,
    SaltyShores,
    #[serde(rename = "DFHStadium_Stormy")]
    DfhStadiumStormy,
    #[serde(rename = "DFHStadium_Day")]
    DfhStadiumDay,
    #[serde(rename = "Mannfield_Stormy")]
    MannfieldStormy,
    #[serde(rename = "Mannfield_Night")]
    MannfieldNight,
    #[serde(rename = "ChampionsField_Day")]
    ChampionsFieldDay,
    #[serde(rename = "BeckwithPark_Stormy")]
    BeckwithParkStormy,
    #[serde(rename = "BeckwithPark_Midnight")]
    BeckwithParkMidnight,
    #[serde(rename = "UrbanCentral_Night")]
    UrbanCentralNight,
    #[serde(rename = "UrbanCentral_Dawn")]
    UrbanCentralDawn,
    #[serde(rename = "UtopiaColiseum_Dusk")]
    UtopiaColiseumDusk,
    #[serde(rename = "DFHStadium_Snowy")]
    DfhStadiumSnowy,
    #[serde(rename = "Mannfield_Snowy")]
    MannfieldSnowy,
    #[serde(rename = "UtopiaColiseum_Snowy")]
    UtopiaColiseumSnowy,
    Badlands,
    #[serde(rename = "Badlands_Night")]
    BadlandsNight,
    TokyoUnderpass,
    Arctagon,
    Pillars,
    Cosmic,
    DoubleGoal,
    Octagon,
    Underpass,
    UtopiaRetro,
    #[serde(rename = "Hoops_DunkHouse")]
    HoopsDunkHouse,
    #[serde(rename = "DropShot_Core707")]
    DropShotCore707,
    ThrowbackStadium,
    ForbiddenTemple,
    RivalsArena,
    #[serde(rename = "Farmstead_Night")]
    FarmsteadNight,
    #[serde(rename = "SaltyShores_Night")]
    SaltyShoresNight,
    NeonFields,
    #[serde(rename = "DFHStadium_Circuit")]
    DFHStadiumCircuit,
    DeadeyeCanyon,
    #[serde(rename = "StarbaseArc_Aftermath")]
    StarbaseArcAftermath,
    #[serde(rename = "Wasteland_Night")]
    WastelandNight,
    BeckwithParkGothamNight,
    #[serde(rename = "ForbiddenTemple_Day")]
    ForbiddenTempleDay,
    #[serde(rename = "UrbanCentral_Haunted")]
    UrbanCentralHaunted,
    #[serde(rename = "ChampionsField_NFL")]
    ChampionsFieldNFL,
    #[serde(rename = "ThrowbackStadium_Snowy")]
    ThrowbackStadiumSnowy,
    Basin,
    Corridor,
    Loophole,
    Galleon,
    GalleonRetro,
    Hourglass,
    Barricade,
    Colossus,
    #[serde(other)]
    Custom(String),
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum GameMode {
    #[default]
    Soccer,
    Hoops,
    Dropshot,
    Hockey,
    Rumble,
    Heatseeker,
    Gridiron,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum ExistingMatchBehavior {
    #[default]
    #[serde(rename = "Restart If Different")]
    RestartIfDifferent,
    Restart,
    #[serde(rename = "Continue And Spawn")]
    ContinueAndSpawn,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum MatchLength {
    #[default]
    #[serde(rename = "5 Minutes")]
    FiveMinutes,
    #[serde(rename = "10 Minutes")]
    TenMinutes,
    #[serde(rename = "20 Minutes")]
    TwentyMinutes,
    Unlimited,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum MaxScore {
    #[default]
    #[serde(rename = "Unlimited")]
    Unlimited,
    #[serde(rename = "1 Goal")]
    OneGoal,
    #[serde(rename = "3 Goals")]
    ThreeGoals,
    #[serde(rename = "5 Goals")]
    FiveGoals,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum Overtime {
    #[default]
    #[serde(rename = "Unlimited")]
    Unlimited,
    #[serde(rename = "+5 Max, First Score")]
    PlusFiveMaxFirstScore,
    #[serde(rename = "+5 Max, Random Team")]
    PlusFiveMaxRandomTeam,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum SeriesLength {
    #[default]
    #[serde(rename = "Unlimited")]
    Unlimited,
    #[serde(rename = "3 Games")]
    ThreeGames,
    #[serde(rename = "5 Games")]
    FiveGames,
    #[serde(rename = "7 Games")]
    SevenGames,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum GameSpeed {
    #[default]
    Default,
    #[serde(rename = "Slo-Mo")]
    SloMo,
    #[serde(rename = "Time Warp")]
    TimeWarp,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum BallMaxSpeed {
    #[default]
    Default,
    #[serde(rename = "Slow")]
    Slow,
    #[serde(rename = "Fast")]
    Fast,
    #[serde(rename = "Super Fast")]
    SuperFast,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum BallType {
    #[default]
    Default,
    Cube,
    Puck,
    Basketball,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum BallWeight {
    #[default]
    Default,
    Light,
    Heavy,
    #[serde(rename = "Super Light")]
    SuperLight,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum BallSize {
    #[default]
    Default,
    Small,
    Large,
    Gigantic,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum BallBounciness {
    #[default]
    Default,
    Low,
    High,
    #[serde(rename = "Super High")]
    SuperHigh,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum BoostAmount {
    #[default]
    Default,
    Unlimited,
    #[serde(rename = "Recharge (Slow)")]
    RechargeSlow,
    #[serde(rename = "Recharge (Fast)")]
    RechargeFast,
    #[serde(rename = "No Boost")]
    NoBoost,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum Rumble {
    #[default]
    None,
    Default,
    Slow,
    Civilized,
    #[serde(rename = "Destruction Derby")]
    DestructionDerby,
    #[serde(rename = "Spring Loaded")]
    SpringLoaded,
    #[serde(rename = "Spikes Only")]
    SpikesOnly,
    #[serde(rename = "Spike Rush")]
    SpikeRush,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum BoostStrength {
    #[default]
    #[serde(rename = "1x")]
    One,
    #[serde(rename = "1.5x")]
    OnePointFive,
    #[serde(rename = "2x")]
    Two,
    #[serde(rename = "10x")]
    Ten,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum Gravity {
    #[default]
    Default,
    Low,
    High,
    #[serde(rename = "Super High")]
    SuperHigh,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum Demolish {
    #[default]
    Default,
    Disabled,
    #[serde(rename = "Friendly Fire")]
    FriendlyFire,
    #[serde(rename = "On Contact")]
    OnContact,
    #[serde(rename = "On Contact (FF)")]
    OnContactFriendlyFire,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, EnumIter)]
pub enum RespawnTime {
    #[default]
    #[serde(rename = "3 Seconds")]
    ThreeSeconds,
    #[serde(rename = "2 Seconds")]
    TwoSeconds,
    #[serde(rename = "1 Second")]
    OneSecond,
    #[serde(rename = "Disable Goal Reset")]
    DisableGoalReset,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Mutators {
    pub match_length_types: Vec<MatchLength>,
    pub max_score_types: Vec<MaxScore>,
    pub overtime_types: Vec<Overtime>,
    pub series_length_types: Vec<SeriesLength>,
    pub game_speed_types: Vec<GameSpeed>,
    pub ball_max_speed_types: Vec<BallMaxSpeed>,
    pub ball_type_types: Vec<BallType>,
    pub ball_weight_types: Vec<BallWeight>,
    pub ball_size_types: Vec<BallSize>,
    pub ball_bounciness_types: Vec<BallBounciness>,
    pub boost_amount_types: Vec<BoostAmount>,
    pub rumble_types: Vec<Rumble>,
    pub boost_strength_types: Vec<BoostStrength>,
    pub gravity_types: Vec<Gravity>,
    pub demolish_types: Vec<Demolish>,
    pub respawn_time_types: Vec<RespawnTime>,
}

impl Default for Mutators {
    fn default() -> Self {
        Self {
            match_length_types: enum_to_vec(),
            max_score_types: enum_to_vec(),
            overtime_types: enum_to_vec(),
            series_length_types: enum_to_vec(),
            game_speed_types: enum_to_vec(),
            ball_max_speed_types: enum_to_vec(),
            ball_type_types: enum_to_vec(),
            ball_weight_types: enum_to_vec(),
            ball_size_types: enum_to_vec(),
            ball_bounciness_types: enum_to_vec(),
            boost_amount_types: enum_to_vec(),
            rumble_types: enum_to_vec(),
            boost_strength_types: enum_to_vec(),
            gravity_types: enum_to_vec(),
            demolish_types: enum_to_vec(),
            respawn_time_types: enum_to_vec(),
        }
    }
}

fn enum_to_vec<E: IntoEnumIterator>() -> Vec<E> {
    E::iter().collect()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatchOptions {
    pub map_types: Vec<MapType>,
    pub game_modes: Vec<GameMode>,
    pub match_behaviours: Vec<ExistingMatchBehavior>,
    pub mutators: Mutators,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            map_types: enum_to_vec(),
            game_modes: enum_to_vec(),
            match_behaviours: enum_to_vec(),
            mutators: Mutators::default(),
        }
    }
}
