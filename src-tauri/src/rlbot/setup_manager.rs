use std::fmt::{Display, Formatter};
use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};
use thiserror::Error;

const ROCKET_LEAGUE_PROGRAM_NAME: &str = if cfg!(windows) { "RocketLeague.exe" } else { "RocketLeague" };
const REQUIRED_ARGS: [&str; 2] = ["-rlbot", "RLBot_ControllerURL=127.0.0.1"];

#[derive(Debug, Error)]
pub struct RLNoBotError(u16);

impl Display for RLNoBotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Please close Rocket League and let RLBot open it for you. Do not start Rocket League yourself. (Rocket League is not running with '{}' and/or on port {} (with '{}:{}'))", REQUIRED_ARGS[0], self.0, REQUIRED_ARGS[1], self.0)
    }
}

pub fn is_rocket_league_running(port: u16) -> Result<bool, RLNoBotError> {
    let system = System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new().with_user()));
    let mut rl_procs = system.processes_by_name(ROCKET_LEAGUE_PROGRAM_NAME);
    let port_arg = format!("{}:{port}", REQUIRED_ARGS[1]);

    let Some(process_info) = rl_procs.next() else {
        return Ok(false);
    };

    let mut has_rlbot_arg = false;
    let mut has_port_arg = false;

    for arg in process_info.cmd().iter().skip(1) {
        if arg == REQUIRED_ARGS[0] {
            has_rlbot_arg = true;
        } else if arg == &port_arg {
            has_port_arg = true;
        }
    }

    if has_port_arg && has_rlbot_arg {
        return Ok(true);
    }

    Err(RLNoBotError(port))
}
