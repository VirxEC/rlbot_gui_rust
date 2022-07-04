import json
import multiprocessing as mp
import os
import platform
import shutil
import sys
from contextlib import contextmanager
from datetime import datetime
from math import pi
from os import path
from pathlib import Path
from traceback import print_exc
from typing import List

from rlbot.gamelaunch.epic_launch import \
    locate_epic_games_launcher_rocket_league_binary
from rlbot.gateway_util import NetworkingRole
from rlbot.matchconfig.loadout_config import LoadoutConfig
from rlbot.matchconfig.match_config import (MatchConfig, MutatorConfig,
                                            PlayerConfig, ScriptConfig)
from rlbot.parsing.agent_config_parser import (
    BOT_CONFIG_LOADOUT_HEADER, BOT_CONFIG_LOADOUT_ORANGE_HEADER,
    BOT_CONFIG_LOADOUT_PAINT_BLUE_HEADER,
    BOT_CONFIG_LOADOUT_PAINT_ORANGE_HEADER, create_looks_configurations,
    load_bot_appearance)
from rlbot.parsing.incrementing_integer import IncrementingInteger
from rlbot.setup_manager import (RocketLeagueLauncherPreference, SetupManager,
                                 try_get_steam_executable_path)
from rlbot.utils import logging_utils
from rlbot.utils.game_state_util import (BallState, CarState, GameInfoState,
                                         GameState, Physics, Rotator, Vector3)
from rlbot.utils.structures.bot_input_struct import PlayerInput
from rlbot.utils.structures.game_data_struct import GameTickPacket
from rlbot.utils.structures.game_data_struct import Physics as PhysicsGTP

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

    if platform.system() == "Windows":
        # This is a very weird issue on Windows only
        # This is the only solution I could find
        # Basically, all bots but the last bot were starting
        # And this somehow fixes that?
        logger.warning("Starting dummy process to ensure bots start")
        proc = mp.Process()
        proc.start()
        proc.join()
        logger.info("Dummy process started, all bots should be running")

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
        print("-|-*|MATCH STARTED|*-|-", flush=True)
    except Exception:
        print_exc()
        print("-|-*|MATCH START FAILED|*-|-", flush=True)

def _physics_to_dict(physics: PhysicsGTP):
    return {
        'location': {
            'x': physics.location.x,
            'y': physics.location.y,
            'z': physics.location.z
        },
        'velocity': {
            'x': physics.velocity.x,
            'y': physics.velocity.y,
            'z': physics.velocity.z
        },
        'angular_velocity': {
            'x': physics.angular_velocity.x,
            'y': physics.angular_velocity.y,
            'z': physics.angular_velocity.z
        },
        'rotation': {
            'pitch': physics.rotation.pitch,
            'yaw': physics.rotation.yaw,
            'roll': physics.rotation.roll,
        },
    }

def fetch_game_tick_packet() -> GameTickPacket:
    global sm
    if sm is None:
        sm = SetupManager()
        sm.connect_to_game()
    game_tick_packet = GameTickPacket()
    sm.game_interface.update_live_data_packet(game_tick_packet)
    # Make Rust GameTickPacket as dict
    return {
        "game_ball": {
            "physics": _physics_to_dict(game_tick_packet.game_ball.physics),
        },
        "game_cars": list({
            "team": car.team,
            "physics": _physics_to_dict(car.physics),
            "boost": car.boost
        } for car in game_tick_packet.game_cars[:game_tick_packet.num_cars]),
        "game_info": {
            "seconds_elapsed": game_tick_packet.game_info.seconds_elapsed,
        },
    }

def dict_to_game_state(state_dict):
    gs = GameState()
    if 'ball' in state_dict:
        gs.ball = BallState()
        if 'physics' in state_dict['ball']:
            gs.ball.physics = dict_to_physics(state_dict['ball']['physics'])
    if 'cars' in state_dict:
        gs.cars = {}
        for index, car in state_dict['cars'].items():
            car_state = CarState()
            if 'physics' in car:
                car_state.physics = dict_to_physics(car['physics'])
            if 'boost_amount' in car:
                car_state.boost_amount = car['boost_amount']
            gs.cars[int(index)] = car_state
    if 'game_info' in state_dict:
        gs.game_info = GameInfoState()
        if 'paused' in state_dict['game_info']:
            gs.game_info.paused = state_dict['game_info']['paused']
        if 'world_gravity_z' in state_dict['game_info']:
            gs.game_info.world_gravity_z = state_dict['game_info']['world_gravity_z']
        if 'game_speed' in state_dict['game_info']:
            gs.game_info.game_speed = state_dict['game_info']['game_speed']
    if 'console_commands' in state_dict:
        gs.console_commands = state_dict['console_commands']
    return gs

def dict_to_physics(physics_dict):
    phys = Physics()
    if 'location' in physics_dict:
        phys.location = dict_to_vec(physics_dict['location'])
    if 'velocity' in physics_dict:
        phys.velocity = dict_to_vec(physics_dict['velocity'])
    if 'angular_velocity' in physics_dict:
        phys.angular_velocity = dict_to_vec(physics_dict['angular_velocity'])
    if 'rotation' in physics_dict:
        phys.rotation = dict_to_rot(physics_dict['rotation'])
    return phys

def dict_to_vec(v):
    vec = Vector3()
    if 'x' in v:
        vec.x = v['x']
    if 'y' in v:
        vec.y = v['y']
    if 'z' in v:
        vec.z = v['z']
    return vec

def dict_to_rot(r):
    rot = Rotator()
    if 'pitch' in r:
        rot.pitch = r['pitch']
    if 'yaw' in r:
        rot.yaw = r['yaw']
    if 'roll' in r:
        rot.roll = r['roll']
    return rot

def set_game_state(state):
    global sm
    if sm is None:
        sm = SetupManager()
        sm.connect_to_game()
    game_state = dict_to_game_state(state)
    sm.game_interface.set_game_state(game_state)

def convert_to_looks_config(looks: dict):
    looks_config = create_looks_configurations()

    def deserialize_category(source: dict, header_name: str):
        header = looks_config.get_header(header_name)
        for key in header.values.keys():
            if key in source:
                header.set_value(key, source[key])

    deserialize_category(looks['blue'], BOT_CONFIG_LOADOUT_HEADER)
    deserialize_category(looks['orange'], BOT_CONFIG_LOADOUT_ORANGE_HEADER)
    deserialize_category(looks['blue'], BOT_CONFIG_LOADOUT_PAINT_BLUE_HEADER)
    deserialize_category(looks['orange'], BOT_CONFIG_LOADOUT_PAINT_ORANGE_HEADER)

    return looks_config

def spawn_car_in_showroom(loadout_config: LoadoutConfig, team: int, showcase_type: str, map_name: str,
                          launcher_prefs: RocketLeagueLauncherPreference):
    match_config = MatchConfig()
    match_config.game_mode = 'Soccer'
    match_config.game_map = map_name
    match_config.instant_start = True
    match_config.existing_match_behavior = 'Continue And Spawn'
    match_config.networking_role = NetworkingRole.none
    match_config.enable_state_setting = True
    match_config.skip_replays = True

    bot_config = PlayerConfig()
    bot_config.bot = True
    bot_config.rlbot_controlled = True
    bot_config.team = team
    bot_config.name = "Showroom"
    bot_config.loadout_config = loadout_config

    match_config.player_configs = [bot_config]
    match_config.mutators = MutatorConfig()
    match_config.mutators.boost_amount = 'Unlimited'
    match_config.mutators.match_length = 'Unlimited'

    global sm
    if sm is None:
        sm = SetupManager()
    sm.connect_to_game(launcher_preference=launcher_prefs)
    sm.load_match_config(match_config)
    sm.start_match()

    game_state = GameState(
        cars={0: CarState(physics=Physics(
            location=Vector3(0, 0, 20),
            velocity=Vector3(0, 0, 0),
            angular_velocity=Vector3(0, 0, 0),
            rotation=Rotator(0, 0, 0)
        ))},
        ball=BallState(physics=Physics(
            location=Vector3(0, 0, -100),
            velocity=Vector3(0, 0, 0),
            angular_velocity=Vector3(0, 0, 0)
        ))
    )
    player_input = PlayerInput()
    team_sign = -1 if team == 0 else 1

    if showcase_type == "boost":
        player_input.boost = True
        player_input.steer = 1
        game_state.cars[0].physics.location.y = -1140
        game_state.cars[0].physics.velocity.x = 2300
        game_state.cars[0].physics.angular_velocity.z = 3.5

    elif showcase_type == "throttle":
        player_input.throttle = 1
        player_input.steer = 0.56
        game_state.cars[0].physics.location.y = -1140
        game_state.cars[0].physics.velocity.x = 1410
        game_state.cars[0].physics.angular_velocity.z = 1.5

    elif showcase_type == "back-center-kickoff":
        game_state.cars[0].physics.location.y = 4608 * team_sign
        game_state.cars[0].physics.rotation.yaw = -0.5 * pi * team_sign

    elif showcase_type == "goal-explosion":
        game_state.cars[0].physics.location.y = -2000 * team_sign
        game_state.cars[0].physics.rotation.yaw = -0.5 * pi * team_sign
        game_state.cars[0].physics.velocity.y = -2300 * team_sign
        game_state.ball.physics.location = Vector3(0, -3500 * team_sign, 93)

    sm.game_interface.update_player_input(player_input, 0)
    sm.game_interface.set_game_state(game_state)

def spawn_car_for_viewing(looks: dict, team: int, showcase_type: str, map_name: str, launcher_prefs: RocketLeagueLauncherPreference):
    looks_config = convert_to_looks_config(looks)
    loadout_config = load_bot_appearance(looks_config, team)
    spawn_car_in_showroom(loadout_config, team, showcase_type, map_name, launcher_prefs)

if __name__ == "__main__":
    try:
        online = True
        while online:
            command = sys.stdin.readline()
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
                if sm is not None:
                    sm.shut_down(time_limit=5, kill_all_pids=True)
                    sm = None
                else:
                    print("There gotta be some setup manager already")
                online = False
            elif params[0] == "fetch-gtp":
                print(f"-|-*|GTP {json.dumps(fetch_game_tick_packet())}|*-|-", flush=True)
            elif params[0] == "set_state":
                state = json.loads(params[1])
                set_game_state(state)
            elif params[0] == "spawn_car_for_viewing":
                config = json.loads(params[1])
                team = int(params[2])
                showcase_type = params[3]
                map_name = params[4]

                preferred_launcher = params[5]
                use_login_tricks = bool(params[6])
                if params[5] != "":
                    rocket_league_exe_path = Path(params[7])
                else:
                    rocket_league_exe_path = None

                spawn_car_for_viewing(config, team, showcase_type, map_name, RocketLeagueLauncherPreference(preferred_launcher, use_login_tricks, rocket_league_exe_path))
    except Exception:
        print_exc()

    if sm is not None:
        sm.shut_down(time_limit=5, kill_all_pids=True)
        sm = None

    print("Closing...", flush=True)
    exit()
