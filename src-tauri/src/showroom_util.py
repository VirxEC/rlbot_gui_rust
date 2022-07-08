from math import pi

from rlbot.gateway_util import NetworkingRole
from rlbot.matchconfig.loadout_config import LoadoutConfig
from rlbot.matchconfig.match_config import (MatchConfig, MutatorConfig,
                                            PlayerConfig)
from rlbot.parsing.agent_config_parser import (
    BOT_CONFIG_LOADOUT_HEADER, BOT_CONFIG_LOADOUT_ORANGE_HEADER,
    BOT_CONFIG_LOADOUT_PAINT_BLUE_HEADER,
    BOT_CONFIG_LOADOUT_PAINT_ORANGE_HEADER, create_looks_configurations,
    load_bot_appearance)
from rlbot.setup_manager import RocketLeagueLauncherPreference, SetupManager
from rlbot.utils.game_state_util import (BallState, CarState, GameInfoState,
                                         GameState, Physics, Rotator, Vector3)
from rlbot.utils.structures.bot_input_struct import PlayerInput
from rlbot.utils.structures.game_data_struct import GameTickPacket
from rlbot.utils.structures.game_data_struct import Physics as PhysicsGTP


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
