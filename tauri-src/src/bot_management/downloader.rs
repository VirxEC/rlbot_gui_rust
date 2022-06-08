use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{read_dir, remove_dir, remove_file, File},
    io::{BufRead, BufReader, Cursor, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use configparser::ini::Ini;
use fs_extra::dir;
use rand::Rng;
use reqwest::{header::USER_AGENT, Client, IntoUrl};
use serde::{Deserialize, Serialize};
use tauri::Window;

use futures_util::StreamExt;

use crate::{bot_management::zip_extract_fixed, ccprintln, ccprintlne, ccprintlnr, get_config_path};

const FOLDER_SUFFIX: &str = "master";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotpackStatus {
    RequiresFullDownload,
    Skipped(String),
    Success(String),
}

fn remove_empty_folders<T: AsRef<Path>>(dir: T) -> Result<(), Box<dyn Error>> {
    let dir = dir.as_ref();

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
        ccprintln(format!("Removed empty folder: {}", dir.display()));
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

async fn download_and_extract_repo_zip<T: IntoUrl, J: AsRef<Path>>(
    window: &Window,
    client: &Client,
    download_url: T,
    local_folder_path: J,
    clobber: bool,
    repo_full_name: &str,
) -> Result<(), reqwest::Error> {
    // download and extract the zip
    let local_folder_path = local_folder_path.as_ref();
    let res = client.get(download_url).send().await?;

    let total_size = (get_repo_size(client, repo_full_name).await.unwrap_or(190_000_000) as f32 * 0.62).round() as usize;
    let mut stream = res.bytes_stream();
    let mut bytes = Vec::with_capacity(total_size);
    let mut last_update = Instant::now();

    while let Some(new_bytes) = stream.next().await {
        // put the new bytes into bytes
        bytes.extend_from_slice(&new_bytes?);

        if last_update.elapsed().as_secs_f32() >= 0.1 {
            let progress = bytes.len() as f32 / total_size as f32 * 100.0;
            if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, "Downloading zip...".to_string())) {
                ccprintlne(format!("Error when updating progress bar: {}", e));
            }
            last_update = Instant::now();
        }
    }

    if clobber && local_folder_path.exists() {
        dir::remove(local_folder_path).unwrap();
    }

    if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(100., "Extracting zip...".to_string())) {
        ccprintlne(format!("Error when updating progress bar: {}", e));
    }

    zip_extract_fixed::extract(Cursor::new(bytes), local_folder_path, false).unwrap();
    Ok(())
}

pub async fn download_repo(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str, update_tag_settings: bool) -> BotpackStatus {
    let client = Client::new();
    let repo_full_name = format!("{}/{}", repo_owner, repo_name);

    let status = download_and_extract_repo_zip(
        window,
        &client,
        &format!("https://github.com/{}/archive/refs/heads/master.zip", repo_full_name),
        checkout_folder,
        true,
        &repo_full_name,
    )
    .await;

    if status.is_ok() && update_tag_settings {
        let latest_release_tag_name = match get_json_from_url(&client, &format!("https://api.github.com/repos/{}/releases/latest", repo_full_name)).await {
            Ok(release) => release["tag_name"].as_str().unwrap().to_string(),
            Err(e) => {
                ccprintlne(e.to_string());
                return BotpackStatus::Success("Downloaded the bot pack, but failed to get the latest release tag.".to_string());
            }
        };

        let config_path = get_config_path();
        let mut config = Ini::new();

        if let Err(e) = config.load(&config_path) {
            ccprintlne(e);
            return BotpackStatus::Success("Downloaded the bot pack, but failed to load GUI's config.".to_string());
        }

        config.set("bot_folder_settings", "incr", Some(latest_release_tag_name));

        if let Err(e) = config.write(config_path) {
            ccprintlne(e.to_string());
            return BotpackStatus::Success("Downloaded the bot pack, but failed to write GUI's config.".to_string());
        }
    }

    match status {
        Ok(_) => BotpackStatus::Success("Downloaded the bot pack!".to_string()),
        Err(e) => {
            ccprintlne(e.to_string());
            BotpackStatus::Skipped("Failed to download the bot pack...".to_string())
        }
    }
}

fn get_current_tag_name() -> Option<u32> {
    let config_path = get_config_path();
    let mut config = Ini::new();
    config.load(&config_path).ok()?;

    config.get("bot_folder_settings", "incr")?.replace("incr-", "").parse::<u32>().ok()
}

fn get_url_from_tag(repo_full_name: &str, tag: u32) -> String {
    format!("https://github.com/{}/releases/download/incr-{}/incremental.zip", repo_full_name, tag)
}

pub async fn is_botpack_up_to_date(repo_full_name: &str) -> bool {
    let current_tag_name = match get_current_tag_name() {
        Some(tag) => tag,
        None => return true,
    };

    let latest_release_tag = match get_json_from_url(&Client::new(), &format!("https://api.github.com/repos/{}/releases/latest", repo_full_name)).await {
        Ok(release) => release["tag_name"].as_str().unwrap().replace("incr-", "").parse::<u32>().unwrap(),
        Err(e) => {
            ccprintlne(format!("{}", e));
            return true;
        }
    };

    latest_release_tag == current_tag_name
}

pub async fn update_bot_pack(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str) -> BotpackStatus {
    let client = Client::new();
    let repo_full_name = format!("{}/{}", repo_owner, repo_name);

    let current_tag_name = match get_current_tag_name() {
        Some(tag) => tag,
        None => return BotpackStatus::RequiresFullDownload,
    };

    let latest_release_tag = match get_json_from_url(&client, &format!("https://api.github.com/repos/{}/releases/latest", repo_full_name)).await {
        Ok(release) => release["tag_name"].as_str().unwrap().replace("incr-", "").parse::<u32>().unwrap(),
        Err(e) => {
            ccprintlne(format!("{}", e));
            return BotpackStatus::Skipped("Failed to get the latest release tag.".to_string());
        }
    };

    if latest_release_tag == current_tag_name {
        ccprintln("The botpack is already up-to-date!".to_string());
        return BotpackStatus::Skipped("The botpack is already up-to-date!".to_string());
    }

    let total_patches = latest_release_tag - current_tag_name;

    if total_patches > 50 {
        return BotpackStatus::RequiresFullDownload;
    }

    let master_folder = format!("{}-{}", repo_name, FOLDER_SUFFIX);
    let local_folder_path = Path::new(checkout_folder).join(master_folder);

    if !local_folder_path.exists() {
        return BotpackStatus::RequiresFullDownload;
    }

    let mut tag = current_tag_name + 1;
    let mut next_download = Some(client.get(get_url_from_tag(&repo_full_name, tag)).send());

    let config_path = get_config_path();
    let mut config = Ini::new();
    if let Err(e) = config.load(&config_path) {
        ccprintlne(format!("Failed to open GUI config: {}", e));
        return BotpackStatus::Skipped("Failed to open GUI config.".to_string());
    }

    let tag_deleted_files_path = local_folder_path.join(".deleted");

    while let Some(future) = next_download {
        ccprintln(format!("Patching in update incr-{}", tag));

        let progress = (tag - current_tag_name) as f32 / total_patches as f32 * 100.;
        if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, format!("Downloading patch incr-{}...", tag))) {
            ccprintlne(format!("Error when updating progress bar: {}", e));
        }

        let download = match future.await {
            Ok(download) => download,
            Err(e) => {
                ccprintlne(format!("Failed to download upgrade zip: {}", e));
                break;
            }
        };

        if tag < latest_release_tag {
            next_download = Some(client.get(get_url_from_tag(&repo_full_name, tag + 1)).send());
        } else {
            next_download = None;
        }

        let progress = progress + 1. / (total_patches as f32 * 2.) * 100.;
        if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, format!("Applying patch incr-{}...", tag))) {
            ccprintlne(format!("Error when updating progress bar: {}", e));
        }

        if let Err(e) = zip_extract_fixed::extract(Cursor::new(&download.bytes().await.unwrap()), local_folder_path.as_path(), false) {
            ccprintlne(format!("Failed to extract upgrade zip: {}", e));
            break;
        }

        match File::open(&tag_deleted_files_path) {
            Ok(file) => {
                let mut last_ok = false;
                let mut count = 0;
                for line in BufReader::new(file).lines().flatten() {
                    let line = line.replace('\0', "").replace('\r', "");
                    if !line.is_empty() {
                        let file_name = local_folder_path.join(line);
                        if let Err(e) = remove_file(&file_name) {
                            ccprintlne(format!("Failed to delete {}: {}", file_name.display(), e));
                            last_ok = false;
                        } else {
                            let text = format!("Deleted {}", file_name.display());
                            if last_ok {
                                ccprintlnr(text);
                            } else {
                                ccprintln(text);
                            }
                            last_ok = true;
                            count += 1;
                        }
                    }
                }

                let text = format!("Deleted {} files", count);
                if last_ok {
                    ccprintlnr(text);
                } else {
                    ccprintln(text);
                }
            }
            Err(e) => {
                ccprintlne(format!("Failed to open .deleted file: {}", e));
                break;
            }
        }

        config.set("bot_folder_settings", "incr", Some(format!("incr-{}", tag)));

        if let Err(e) = config.write(&config_path) {
            ccprintlne(e.to_string());
        }

        tag += 1;

        if tag_deleted_files_path.exists() {
            if let Err(e) = remove_file(&tag_deleted_files_path) {
                ccprintlne(format!("Failed to delete {}: {}", tag_deleted_files_path.display(), e));
                break;
            }
        }
    }

    if let Err(e) = remove_empty_folders(local_folder_path) {
        ccprintlne(format!("Failed to remove empty folders: {}", e));
    }

    if tag - 1 == latest_release_tag {
        BotpackStatus::Success("Updated the botpack!".to_string())
    } else {
        BotpackStatus::Skipped("Failed to update the botpack...".to_string())
    }
}

pub struct MapPackUpdater {
    full_path: PathBuf,
    repo_owner: String,
    repo_name: String,
    client: Client,
}

impl MapPackUpdater {
    pub fn new<T: AsRef<Path>>(location: T, repo_owner: String, repo_name: String) -> Self {
        Self {
            full_path: location.as_ref().join(format!("{}-{}", &repo_name, "main")),
            repo_owner,
            repo_name,
            client: Client::new(),
        }
    }

    /// For a map pack, gets you the index.json data
    pub fn get_map_index(&self) -> Option<serde_json::Value> {
        let index_path = self.full_path.join("index.json");

        if index_path.exists() {
            let mut file = File::open(index_path).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            return Some(serde_json::from_str(&contents).unwrap());
        }

        None
    }

    /// Compares the old_index with current index and for any
    /// maps that have updated the revision, we grab them
    /// from the latest revision
    pub async fn needs_update(&self) -> BotpackStatus {
        let index = match self.get_map_index() {
            Some(index) => index,
            None => return BotpackStatus::RequiresFullDownload,
        };

        let revision = index["revision"].as_u64().unwrap();

        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", self.repo_owner, self.repo_name);

        let latest_release = match get_json_from_url(&self.client, &url).await {
            Ok(latest_release) => latest_release,
            Err(e) => {
                ccprintlne(format!("Failed to get latest release: {}", e));
                return BotpackStatus::Skipped("Failed to get latest release".to_string());
            }
        };

        let latest_revision = latest_release["tag_name"].as_str().unwrap()[1..].parse::<u64>().unwrap();

        if latest_revision > revision {
            BotpackStatus::RequiresFullDownload
        } else {
            ccprintln("Map pack is already up-to-date!".to_string());
            BotpackStatus::Skipped("Map pack is already up-to-date!".to_string())
        }
    }

    fn extract_maps_from_index(index: serde_json::Value) -> HashMap<String, u64> {
        index["maps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|map| (map["path"].as_str().unwrap().to_string(), map["revision"].as_u64().unwrap()))
            .collect::<HashMap<String, u64>>()
    }

    /// Compares the old_index with current index and for any
    /// maps that have updated the revision, we grab them
    /// from the latest revision
    pub async fn hydrate_map_pack(&self, old_index: Option<serde_json::Value>) {
        let new_maps = match self.get_map_index() {
            Some(index) => Self::extract_maps_from_index(index),
            None => {
                ccprintlne("Failed to get index.json".to_string());
                return;
            }
        };

        let old_maps = match old_index {
            Some(index) => Self::extract_maps_from_index(index),
            None => HashMap::new(),
        };

        let mut to_fetch = HashSet::new();
        for (path, revision) in new_maps.iter() {
            if !old_maps.contains_key(path) || old_maps[path] < *revision {
                to_fetch.insert(path.to_string());
            }
        }

        if to_fetch.is_empty() {
            return;
        }

        let mut filename_to_path = HashMap::new();
        for path in to_fetch.iter() {
            let filename = Path::new(path).file_name().unwrap().to_str().unwrap();
            filename_to_path.insert(filename.to_string(), path.to_string());
        }

        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", self.repo_owner, self.repo_name);

        let latest_release = match get_json_from_url(&self.client, &url).await {
            Ok(latest_release) => latest_release,
            Err(e) => {
                ccprintlne(format!("Failed to get latest release: {}", e));
                return;
            }
        };

        for asset in latest_release["assets"].as_array().unwrap() {
            let asset_name = asset["name"].as_str().unwrap();
            if let Err(e) = self.download_asset(asset, asset_name, &filename_to_path, &self.full_path).await {
                ccprintlne(format!("Failed to download asset {}: {}", asset_name, e));
            }
        }
    }

    async fn download_asset<T: AsRef<Path>>(
        &self,
        asset: &serde_json::Value,
        asset_name: &str,
        filename_to_path: &HashMap<String, String>,
        full_path: T,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(local_path) = filename_to_path.get(asset_name) {
            let target_path = full_path.as_ref().join(local_path);
            ccprintln(format!("Will fetch updated map {}", asset_name));

            let url = asset["browser_download_url"].as_str().unwrap();
            let resp = self.client.get(url).send().await?.bytes().await?;
            let mut file = File::create(target_path)?;
            file.write_all(&resp)?;
        }

        Ok(())
    }
}
