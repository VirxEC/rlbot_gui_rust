import json
import multiprocessing as mp
import sys
from pathlib import Path
from traceback import print_exc

from rlbot.setup_manager import RocketLeagueLauncherPreference

from .showroom_util import (fetch_game_tick_packet, set_game_state,
                            spawn_car_for_viewing)
from .start_match_util import start_match_helper

if __name__ == "__main__":
    mp.set_start_method("spawn")
    
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
