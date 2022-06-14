import json
import os
import shutil
import sys
from contextlib import contextmanager
from datetime import datetime
from os import path
from pathlib import Path
from traceback import print_exc
from typing import List

from rlbot.gamelaunch.epic_launch import \
    locate_epic_games_launcher_rocket_league_binary
from rlbot.matchconfig.match_config import (MatchConfig, MutatorConfig,
                                            PlayerConfig, ScriptConfig)
from rlbot.parsing.incrementing_integer import IncrementingInteger
from rlbot.setup_manager import (RocketLeagueLauncherPreference, SetupManager,
                                 try_get_steam_executable_path)
from rlbot.utils import logging_utils

sm: SetupManager = None

CUSTOM_MAP_TARGET = {"filename": "Labs_Utopia_P.upk", "game_map": "UtopiaRetro"}

logger = logging_utils.get_logger("custom_maps")


def create_player_config(bot: dict, human_index_tracker: IncrementingInteger):
    player_config = PlayerConfig()
    player_config.bot = bot['runnable_type'] in ('rlbot', 'psyonix')
    player_config.rlbot_controlled = bot['runnable_type'] in ('rlbot', 'party_member_bot')
    player_config.bot_skill = bot['skill']
    player_config.human_index = 0 if player_config.bot else human_index_tracker.increment()
    player_config.name = bot['name']
    player_config.team = int(bot['team'])
    if 'path' in bot and bot['path']:
        player_config.config_path = bot['path']
    return player_config

def create_script_config(script):
    return ScriptConfig(script['path'])


def get_fresh_setup_manager():
    global sm
    if sm is not None:
        try:
            sm.shut_down()
        except Exception as e:
            print(e)
    sm = SetupManager()
    return sm


@contextmanager
def prepare_custom_map(custom_map_file: str, rl_directory: str):
    """
    Provides a context manager. It will swap out the custom_map_file
    for an existing map in RL and it will return the `game_map`
    name that should be used in a MatchConfig.
    Once the context is left, the original map is replaced back.
    The context should be left as soon as the match has started
    """

    # check if there metadata for the custom file
    expected_config_name = "_" + path.basename(custom_map_file)[:-4] + ".cfg"
    config_path = path.join(path.dirname(custom_map_file), expected_config_name)
    additional_info = {
        "original_path": custom_map_file,
    }
    if path.exists(config_path):
        additional_info["config_path"] = config_path


    real_map_file = path.join(rl_directory, CUSTOM_MAP_TARGET["filename"])
    timestamp = datetime.now().strftime("%Y-%m-%dT%H-%M-%S")
    temp_filename = real_map_file + "." + timestamp

    shutil.copy2(real_map_file, temp_filename)
    logger.info("Copied real map to %s", temp_filename)
    shutil.copy2(custom_map_file, real_map_file)
    logger.info("Copied custom map from %s", custom_map_file)

    try:
        yield CUSTOM_MAP_TARGET["game_map"], additional_info
    finally:
        os.replace(temp_filename, real_map_file)
        logger.info("Reverted real map to %s", real_map_file)


def identify_map_directory(launcher_pref: RocketLeagueLauncherPreference):
    """Find RocketLeague map directory"""
    final_path = None
    if launcher_pref.preferred_launcher == RocketLeagueLauncherPreference.STEAM:
        steam = try_get_steam_executable_path()
        suffix = r"steamapps\common\rocketleague\TAGame\CookedPCConsole"
        if not steam:
            return None

        # TODO: Steam can install RL on a different disk. Need to
        # read libraryfolders.vdf to detect this situation
        # It's a human-readable but custom format so not trivial to parse

        final_path = path.join(path.dirname(steam), suffix)
    else:
        rl_executable = locate_epic_games_launcher_rocket_league_binary()
        suffix = r"TAGame\CookedPCConsole"
        if not rl_executable:
            return None

        # Binaries/Win64/ is what we want to strip off
        final_path = path.join(path.dirname(rl_executable), "..", "..", suffix)

    if not path.exists(final_path):
        logger.warning("%s - directory doesn't exist", final_path)
        return None
    return final_path


def setup_match(
    setup_manager: SetupManager, match_config: MatchConfig, launcher_pref: RocketLeagueLauncherPreference
):
    """Starts the match and bots. Also detects and handles custom maps"""

    map_file = match_config.game_map
    if map_file.endswith('.upk') or map_file.endswith('.udk'):
        rl_directory = identify_map_directory(launcher_pref)

        if not rl_directory:
            raise Exception("Couldn't find path to Rocket League maps folder")

        with prepare_custom_map(map_file, rl_directory) as (map_file, metadata):
            match_config.game_map = map_file
            if "config_path" in metadata:
                config_path = metadata["config_path"]
                match_config.script_configs.append(
                    create_script_config({'path': config_path}))
                print(f"Will load custom script for map {config_path}")

    setup_manager.early_start_seconds = 5
    setup_manager.connect_to_game(launcher_preference=launcher_pref)

    # Loading the setup manager's game interface just as a quick fix because story mode uses it. Ideally story mode
    # should now make its own game interface to use.
    setup_manager.game_interface.load_interface(wants_ball_predictions=False, wants_quick_chat=False, wants_game_messages=False)
    setup_manager.load_match_config(match_config)
    setup_manager.launch_early_start_bot_processes()
    setup_manager.start_match()
    setup_manager.launch_bot_processes()

def start_match_helper(bot_list: List[dict], match_settings: dict, launcher_prefs: RocketLeagueLauncherPreference):
    print(f"Bot list: {bot_list}")
    print(f"Match settings: {match_settings}")
    print(f"Launcher preferences: {launcher_prefs}")

    match_config = MatchConfig()
    match_config.game_mode = match_settings['game_mode']
    match_config.game_map = match_settings['map']
    match_config.skip_replays = match_settings['skip_replays']
    match_config.instant_start = match_settings['instant_start']
    match_config.enable_lockstep = match_settings['enable_lockstep']
    match_config.enable_rendering = match_settings['enable_rendering']
    match_config.enable_state_setting = match_settings['enable_state_setting']
    match_config.auto_save_replay = match_settings['auto_save_replay']
    match_config.existing_match_behavior = match_settings['match_behavior']
    match_config.mutators = MutatorConfig()

    mutators = match_settings['mutators']
    match_config.mutators.match_length = mutators['match_length']
    match_config.mutators.max_score = mutators['max_score']
    match_config.mutators.overtime = mutators['overtime']
    match_config.mutators.series_length = mutators['series_length']
    match_config.mutators.game_speed = mutators['game_speed']
    match_config.mutators.ball_max_speed = mutators['ball_max_speed']
    match_config.mutators.ball_type = mutators['ball_type']
    match_config.mutators.ball_weight = mutators['ball_weight']
    match_config.mutators.ball_size = mutators['ball_size']
    match_config.mutators.ball_bounciness = mutators['ball_bounciness']
    match_config.mutators.boost_amount = mutators['boost_amount']
    match_config.mutators.rumble = mutators['rumble']
    match_config.mutators.boost_strength = mutators['boost_strength']
    match_config.mutators.gravity = mutators['gravity']
    match_config.mutators.demolish = mutators['demolish']
    match_config.mutators.respawn_time = mutators['respawn_time']

    human_index_tracker = IncrementingInteger(0)
    match_config.player_configs = [create_player_config(bot, human_index_tracker) for bot in bot_list]
    match_config.script_configs = [create_script_config(script) for script in match_settings['scripts']]

    # these fancy prints to stderr will not get printed to the console
    # the Rust port of the RLBotGUI will capture it and fire a tauri event

    sm = get_fresh_setup_manager()
    try:
        setup_match(sm, match_config, launcher_prefs)
    except Exception:
        print_exc()
        print("-|-*|MATCH START FAILED|*-|-", file=sys.stderr)
        return

    print("-|-*|MATCH STARTED|*-|-", file=sys.stderr)

def shut_down():
    if sm is not None:
        sm.shut_down(time_limit=5, kill_all_pids=True)
    else:
        print("There gotta be some setup manager already")

if __name__ == "__main__":
    while True:
        command = input()
        try:
            params = command.split(" | ")
            if params[0] == "start_match":
                bot_list = json.loads(params[1])
                match_settings = json.loads(params[2])

                preferred_launcher = params[3]
                use_login_tricks = bool(params[4])
                if params[5] != "":
                    rocket_league_exe_path = Path(params[5])
                else:
                    rocket_league_exe_path = None

                start_match_helper(bot_list, match_settings, RocketLeagueLauncherPreference(preferred_launcher, use_login_tricks, rocket_league_exe_path))
            elif params[0] == "shut_down":
                break
        except Exception:
            print_exc()

    shut_down()
