use crate::ccprintln;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use tauri::Window;

pub const IDEAL_RLBOT_PORT: u16 = 23233;
const EXECUTABLE_NAME: &str = if cfg!(windows) {
    "RLBot.exe"
} else if cfg!(target_os = "macos") {
    "RLBot_mac"
} else {
    "RLBot"
};

pub fn find_existing_process(window: &Window) -> Option<u16> {
    let system = System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));

    if let Some(process_info) = system.processes_by_name(EXECUTABLE_NAME).next() {
        if let Some(arg) = process_info.cmd().get(1) {
            let port = arg
                .parse::<u16>()
                .map_err(|e| {
                    ccprintln(window, e.to_string());
                })
                .ok()?;
            ccprintln!(window, "Found existing RLBot process listening on port {port}");
            return Some(port);
        }
    }

    ccprintln(window, "No existing RLBot process found...");
    None
}

pub fn kill_existing_processes(window: &Window) {
    let system = System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));

    let mut found = false;
    // there might be multiple processes, so just be able to kill them all encase
    for process_info in system.processes_by_name(EXECUTABLE_NAME) {
        ccprintln(window, "Killing existing RLBot process");
        process_info.kill();
        found = true;
    }

    if !found {
        ccprintln(window, "No existing RLBot process found");
    }
}
