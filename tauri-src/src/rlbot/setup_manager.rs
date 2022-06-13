use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

const ROCKET_LEAGUE_PROGRAM_NAME: &str = if cfg!(windows) { "RocketLeague.exe" } else { "RocketLeague" };
const REQUIRED_ARGS: [&str; 2] = ["-rlbot", "RLBot_ControllerURL=127.0.0.1"];

pub fn is_rocket_league_running(port: u16) -> Result<bool, String> {
    let system = System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));
    let mut rl_procs = system.processes_by_name(ROCKET_LEAGUE_PROGRAM_NAME);
    let port_arg = format!("{}:{}", REQUIRED_ARGS[1], port);

    match rl_procs.next() {
        Some(process_info) => {
            let cmd = process_info.cmd();
            if cmd.len() > 1 {
                let mut has_rlbot_arg = false;
                let mut has_port_arg = false;
                for arg in cmd.iter().skip(1) {
                    dbg!(arg);
                    if arg == REQUIRED_ARGS[0] {
                        has_rlbot_arg = true;
                    } else if arg == &port_arg {
                        has_port_arg = true;
                    }
                }

                if has_port_arg && has_rlbot_arg {
                    return Ok(true);
                }
            }

            Err(format!(
                "Rocket League is not running with '{}' and/or on port {} (with '{}').\nPlease close Rocket League and let us start it for you instead.",
                REQUIRED_ARGS[0], port, port_arg
            ))
        }
        None => Ok(false),
    }
}
