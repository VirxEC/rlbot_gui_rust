use std::{
    error::Error,
    fs::{read_dir, remove_dir},
    io::Cursor,
    path::Path,
    time::Instant,
};

use configparser::ini::Ini;
use rand::Rng;
use reqwest::{header::USER_AGENT, Client, IntoUrl};
use serde::{Deserialize, Serialize};
use tauri::Window;

use futures_util::StreamExt;

use crate::{ccprintln, get_config_path};

const MAPPACK_DIR: &str = "RLBotMapPack-master";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotpackStatus {
    RequiresFullDownload,
    Skipped,
    Success,
}

fn remove_empty_folders(dir: &Path) -> Result<(), Box<dyn Error>> {
    // remove any empty sub folders
    for entry in read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_empty_folders(&path)?;
        }
    }

    // remove the folder if it is empty
    if dir.read_dir()?.next().is_none() {
        remove_dir(dir)?;
    }

    Ok(())
}

async fn get_json_from_url(client: &Client, url: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    // get random string
    let user_agent: [char; 8] = rand::thread_rng().gen();
    Ok(client.get(url).header(USER_AGENT, user_agent.iter().collect::<String>()).send().await?.json().await?)
}

/// Returns Size of the repository in bytes, or None if the API call fails.
///
/// Call GitHub API to get an estimate size of a GitHub repository.
///
/// * `repo_full_name` Full name of a repository. Example: 'RLBot/RLBotPack'
async fn get_repo_size(client: &Client, repo_full_name: &str) -> Result<u64, Box<dyn Error>> {
    let data = get_json_from_url(client, &format!("https://api.github.com/repos/{}", repo_full_name)).await?;
    Ok(data["size"].as_u64().unwrap() * 1000)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProgressBarUpdate {
    pub percent: f32,
    pub status: String,
}

impl ProgressBarUpdate {
    pub const fn new(percent: f32, status: String) -> Self {
        Self { percent, status }
    }
}

async fn download_and_extract_zip<T: IntoUrl, J: AsRef<Path>>(window: &Window, client: &Client, download_url: T, local_folder_path: J, clobber: bool, repo_full_name: &str) -> BotpackStatus {
    // download and extract the zip
    let local_folder_path = local_folder_path.as_ref();

    if let Ok(res) = client.get(download_url).send().await {
        let total_size = get_repo_size(client, repo_full_name).await.unwrap_or(190_000_000) as f32 * 0.62;
        let mut stream = res.bytes_stream();
        let mut bytes = Vec::new();
        let mut last_update = Instant::now();

        while let Some(new_bytes) = stream.next().await {
            if let Ok(new_bytes) = new_bytes {
                // put the new bytes into bytes
                bytes.extend_from_slice(&new_bytes);

                if last_update.elapsed().as_secs_f32() >= 0.1 {
                    let progress = bytes.len() as f32 / total_size * 100.0;
                    if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, "Downloading zip...".to_string())) {
                        ccprintln(format!("Error when updating progress bar: {}", e));
                    }
                    last_update = Instant::now();
                }
            } else {
                return BotpackStatus::Skipped;
            }
        }

        if clobber && local_folder_path.exists() && fs_extra::dir::remove(local_folder_path).is_err() {
            return BotpackStatus::Skipped;
        }

        if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(100., "Extracting zip...".to_string())) {
            ccprintln(format!("Error when updating progress bar: {}", e));
        }
        
        zip_extract::extract(Cursor::new(bytes), local_folder_path, false).unwrap();
        BotpackStatus::Success
    } else {
        BotpackStatus::Skipped
    }
}

pub async fn download_repo(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str, update_tag_settings: bool) -> BotpackStatus {
    let client = reqwest::Client::new();
    let repo_full_name = format!("{}/{}", repo_owner, repo_name);

    let status = download_and_extract_zip(
        window,
        &client,
        &format!("https://github.com/{}/archive/refs/heads/master.zip", repo_full_name),
        checkout_folder,
        true,
        &repo_full_name,
    )
    .await;

    if status == BotpackStatus::Success && update_tag_settings {
        let latest_release_tag_name = match get_json_from_url(&client, &format!("https://api.github.com/repos/{}/releases/latest", repo_full_name)).await {
            Ok(release) => release["tag_name"].as_str().unwrap().to_string(),
            Err(e) => {
                ccprintln(format!("{}", e));
                return BotpackStatus::Skipped;
            }
        };

        let config_path = get_config_path();
        let mut config = Ini::new();

        if let Err(e) = config.load(&config_path) {
            ccprintln(e);
            return BotpackStatus::Success;
        }

        config.set("bot_folder_settings", "incr", Some(latest_release_tag_name));

        if let Err(e) = config.write(config_path) {
            ccprintln(e.to_string());
            return BotpackStatus::Success;
        }
    }

    status
}
