use serde::{Deserialize, Serialize};
use tini::{Error, Ini};

pub const BOT_CONFIG_LOADOUT_HEADER: &str = "Bot Loadout";
pub const BOT_CONFIG_LOADOUT_ORANGE_HEADER: &str = "Bot Loadout Orange";
pub const BOT_CONFIG_LOADOUT_PAINT_BLUE_HEADER: &str = "Bot Paint Blue";
pub const BOT_CONFIG_LOADOUT_PAINT_ORANGE_HEADER: &str = "Bot Paint Orange";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BotTeamLooksConfig {
    pub team_color_id: String,
    pub custom_color_id: String,
    pub car_id: String,
    pub decal_id: String,
    pub wheels_id: String,
    pub boost_id: String,
    pub antenna_id: String,
    pub hat_id: String,
    pub paint_finish_id: String,
    pub custom_finish_id: String,
    pub engine_audio_id: String,
    pub trails_id: String,
    pub goal_explosion_id: String,
    pub primary_color_lookup: String,
    pub secondary_color_lookup: String,
    pub car_paint_id: String,
    pub decal_paint_id: String,
    pub wheels_paint_id: String,
    pub boost_paint_id: String,
    pub antenna_paint_id: String,
    pub hat_paint_id: String,
    pub trails_paint_id: String,
    pub goal_explosion_paint_id: String,
}

impl BotTeamLooksConfig {
    pub fn from_path(loadout_header: &str, paint_header: &str, path: &str) -> Result<Self, Error> {
        let config = Ini::from_file(path)?;

        let team_color_id = config.get::<String>(loadout_header, "team_color_id").unwrap_or_default();
        let custom_color_id = config.get::<String>(loadout_header, "custom_color_id").unwrap_or_default();
        let car_id = config.get::<String>(loadout_header, "car_id").unwrap_or_default();
        let decal_id = config.get::<String>(loadout_header, "decal_id").unwrap_or_default();
        let wheels_id = config.get::<String>(loadout_header, "wheels_id").unwrap_or_default();
        let boost_id = config.get::<String>(loadout_header, "boost_id").unwrap_or_default();
        let antenna_id = config.get::<String>(loadout_header, "antenna_id").unwrap_or_default();
        let hat_id = config.get::<String>(loadout_header, "hat_id").unwrap_or_default();
        let paint_finish_id = config.get::<String>(loadout_header, "paint_finish_id").unwrap_or_default();
        let custom_finish_id = config.get::<String>(loadout_header, "custom_finish_id").unwrap_or_default();
        let engine_audio_id = config.get::<String>(loadout_header, "engine_audio_id").unwrap_or_default();
        let trails_id = config.get::<String>(loadout_header, "trails_id").unwrap_or_default();
        let goal_explosion_id = config.get::<String>(loadout_header, "goal_explosion_id").unwrap_or_default();
        let primary_color_lookup = config.get::<String>(loadout_header, "primary_color_lookup").unwrap_or_default();
        let secondary_color_lookup = config.get::<String>(loadout_header, "secondary_color_lookup").unwrap_or_default();

        let car_paint_id = config.get::<String>(paint_header, "car_paint_id").unwrap_or_default();
        let decal_paint_id = config.get::<String>(paint_header, "decal_paint_id").unwrap_or_default();
        let wheels_paint_id = config.get::<String>(paint_header, "wheels_paint_id").unwrap_or_default();
        let boost_paint_id = config.get::<String>(paint_header, "boost_paint_id").unwrap_or_default();
        let antenna_paint_id = config.get::<String>(paint_header, "antenna_paint_id").unwrap_or_default();
        let hat_paint_id = config.get::<String>(paint_header, "hat_paint_id").unwrap_or_default();
        let trails_paint_id = config.get::<String>(paint_header, "trails_paint_id").unwrap_or_default();
        let goal_explosion_paint_id = config.get::<String>(paint_header, "goal_explosion_paint_id").unwrap_or_default();

        Ok(Self {
            team_color_id,
            custom_color_id,
            car_id,
            decal_id,
            wheels_id,
            boost_id,
            antenna_id,
            hat_id,
            paint_finish_id,
            custom_finish_id,
            engine_audio_id,
            trails_id,
            goal_explosion_id,
            primary_color_lookup,
            secondary_color_lookup,
            car_paint_id,
            decal_paint_id,
            wheels_paint_id,
            boost_paint_id,
            antenna_paint_id,
            hat_paint_id,
            trails_paint_id,
            goal_explosion_paint_id,
        })
    }

    pub fn save_to_config(&self, config: Ini, loadout_header: &str, paint_header: &str) -> Ini {
        config
            .section(loadout_header)
            .item("team_color_id", &self.team_color_id)
            .item("custom_color_id", &self.custom_color_id)
            .item("car_id", &self.car_id)
            .item("decal_id", &self.decal_id)
            .item("wheels_id", &self.wheels_id)
            .item("boost_id", &self.boost_id)
            .item("antenna_id", &self.antenna_id)
            .item("hat_id", &self.hat_id)
            .item("paint_finish_id", &self.paint_finish_id)
            .item("custom_finish_id", &self.custom_finish_id)
            .item("engine_audio_id", &self.engine_audio_id)
            .item("trails_id", &self.trails_id)
            .item("goal_explosion_id", &self.goal_explosion_id)
            .item("primary_color_lookup", &self.primary_color_lookup)
            .item("secondary_color_lookup", &self.secondary_color_lookup)
            .section(paint_header)
            .item("car_paint_id", &self.car_paint_id)
            .item("decal_paint_id", &self.decal_paint_id)
            .item("wheels_paint_id", &self.wheels_paint_id)
            .item("boost_paint_id", &self.boost_paint_id)
            .item("antenna_paint_id", &self.antenna_paint_id)
            .item("hat_paint_id", &self.hat_paint_id)
            .item("trails_paint_id", &self.trails_paint_id)
            .item("goal_explosion_paint_id", &self.goal_explosion_paint_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BotLooksConfig {
    pub blue: BotTeamLooksConfig,
    pub orange: BotTeamLooksConfig,
}

impl BotLooksConfig {
    pub fn from_path(path: &str) -> Result<Self, Error> {
        Ok(Self {
            blue: BotTeamLooksConfig::from_path(BOT_CONFIG_LOADOUT_HEADER, BOT_CONFIG_LOADOUT_PAINT_BLUE_HEADER, path)?,
            orange: BotTeamLooksConfig::from_path(BOT_CONFIG_LOADOUT_ORANGE_HEADER, BOT_CONFIG_LOADOUT_PAINT_ORANGE_HEADER, path)?,
        })
    }

    pub fn save_to_path(&self, path: &str) {
        let config = self.blue.save_to_config(Ini::new(), BOT_CONFIG_LOADOUT_HEADER, BOT_CONFIG_LOADOUT_PAINT_BLUE_HEADER);
        let config = self.orange.save_to_config(config, BOT_CONFIG_LOADOUT_ORANGE_HEADER, BOT_CONFIG_LOADOUT_PAINT_ORANGE_HEADER);

        config.to_file(path).unwrap();
    }
}
