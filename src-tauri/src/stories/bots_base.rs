#[allow(clippy::too_many_lines)]
pub fn json() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({
        "psyonix-pro": {
            "name": "Psyonix Pro",
            "type": "psyonix",
            "skill": 0.5
        },
        "psyonix-allstar-name": {
            "name": "Psyonix Allstar",
            "type": "psyonix",
            "skill": 1
        },
        "adversity": {
            "name": "AdversityBot",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "ReliefBotFamily", "README", "adversity_bot.cfg"]
        },
        "airbud": {
            "name": "Airbud",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "ReliefBotFamily", "README", "air_bud.cfg"]
        },
        "reliefbot": {
            "name": "ReliefBot",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "ReliefBotFamily", "README", "relief_bot.cfg"]
        },
        "sdc": {
            "name": "Self-Driving Car",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Self-driving car", "self-driving-car.cfg"]
        },
        "kamael": {
            "name": "Kamael",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Kamael_family", "Kamael.cfg"]
        },
        "botimus": {
            "name": "BoltimusPrime",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Botimus&Bumblebee", "botimus.cfg"]
        },
        "bumblebee": {
            "name": "Bumblebee",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Botimus&Bumblebee", "bumblebee.cfg"]
        },
        "baf": {
            "name": "Flying Panda",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "blind_and_deaf", "_story_mode_bot.cfg"]
        },
        "tbd": {
            "name": "Psyonix Allstar",
            "type": "psyonix",
            "skill": 1
        },
        "skybot": {
            "name": "Skybot",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Skybot", "SkyBot.cfg"]
        },
        "wildfire": {
            "name": "Wildfire",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Wildfire_Lightfall_Fix", "python", "wildfire.cfg"]
        },
        "diablo": {
            "name": "Diablo",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Diablo", "diablo.cfg"]
        },
        "rashbot": {
            "name": "rashBot",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "MarvinBots", "rashBot", "rashBot.cfg"]
        },
        "stick": {
            "name": "Stick",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "MarvinBots", "Stick", "stick.cfg"]
        },
        "leaf": {
            "name": "Leaf",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "MarvinBots", "Leaf", "leaf.cfg"]
        },
        "lanfear": {
            "name": "Lanfear",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "The Forsaken", "Lanfear.cfg"]
        },
        "phoenix": {
            "name": "Phoenix",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "PhoenixCS", "phoenix.cfg"]
        },
        "atlas": {
            "name": "Atlas",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Atlas_Wintertide_Patch", "AtlasAgent", "Atlas.cfg"]
        },
        "sniper": {
            "name": "Sniper",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Sniper", "sniper.cfg"]
        },
        "snek": {
            "name": "Snek",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Snek", "snek.cfg"]
        },
        "nombot": {
            "name": "NomBot",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "DomNomNom", "NomBot_v1.0", "NomBot_v1.cfg"]
        },
        "beast": {
            "name": "Beast from the East",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "beastbot", "beastbot.cfg"]
        },
        "cryo": {
            "name": "Codename Cryo",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Codename_Cryo", "Codename_Cryo.cfg"]
        },
        "penguin": {
            "name": "PenguinBot",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "PenguinBot", "penguin_config.cfg"]
        },
        "peter": {
            "name": "St. Peter",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Kamael_family", "peter.cfg"]
        },
        "invisibot": {
            "name": "Invisibot",
            "type": "rlbot",
            "path": ["$RLBOTPACKROOT", "RLBotPack", "Invisibot", "src", "invisibot.cfg"]
        }
    })
    .as_object()
    .unwrap()
    .clone()
}
