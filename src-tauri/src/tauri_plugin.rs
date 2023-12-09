use crate::{commands::shut_down_match_handler, InternalConsoleError, CONSOLE_TEXT_OUT_QUEUE};
use tauri::{
    async_runtime::block_on as tauri_block_on,
    plugin::{Builder as PluginBuilder, TauriPlugin},
    RunEvent, Runtime,
};

pub fn log_text(text: String) -> Result<(), InternalConsoleError> {
    println!("{text}");
    CONSOLE_TEXT_OUT_QUEUE
        .read()
        .map_err(|_| InternalConsoleError::Poisoned("CONSOLE_TEXT_OUT_QUEUE"))?
        .as_ref()
        .ok_or_else(|| InternalConsoleError::None("CONSOLE_TEXT_OUT_QUEUE"))?
        .send(text)?;

    Ok(())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    PluginBuilder::new("eventhandler")
        .on_event(|_, event| {
            if matches!(event, RunEvent::Exit) {
                if let Err(e) = tauri_block_on(async { shut_down_match_handler().await }) {
                    if let Err(e) = log_text(e.to_string()) {
                        println!("{e}");
                    }
                }
            }
        })
        .build()
}
