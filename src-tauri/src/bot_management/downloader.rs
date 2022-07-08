use super::cfg_helper::load_cfg;
use crate::{bot_management::zip_extract_fixed, ccprintln, ccprintlne, ccprintlnr, get_config_path, load_gui_config};
use fs_extra::dir;
use futures_util::StreamExt;
use rand::Rng;
use reqwest::{header::USER_AGENT, Client, IntoUrl};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{read_dir, remove_dir, remove_file, File},
    io::{BufRead, BufReader, Cursor, Error as IoError, ErrorKind as IoErrorKind, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use tauri::Window;
use tokio::task;

const FOLDER_SUFFIX: &str = "master";

/// Represents the action taken after a function
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BotpackStatus {
    /// Update did not run, a full download must be ran
    RequiresFullDownload,
    /// There is no update / there was an error when updating
    Skipped(String),
    /// Update / full download was successful
    Success(String),
}

/// Remove all empty folders in a directory, returns an error if something went wrong
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `dir`: The directory to target
fn remove_empty_folders<T: AsRef<Path>>(window: &Window, dir: T) -> Result<(), Box<dyn Error>> {
    let dir = dir.as_ref();

    // remove any empty sub folders
    for entry in read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_empty_folders(window, &path)?;
        }
    }

    // remove the folder if it is empty
    if dir.read_dir()?.next().is_none() {
        remove_dir(dir)?;
        ccprintln(window, format!("Removed empty folder: {}", dir.display()));
    }

    Ok(())
}

/// Get a JSON file from a URL and do a generic parse, returning an error if something went wrong
///
/// # Arguments
///
/// * `client`: The client to use to make the request
/// * `url`: The URL to get the JSON from
async fn get_json_from_url(client: &Client, url: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    // get random string
    let user_agent: [char; 8] = rand::thread_rng().gen();
    Ok(client.get(url).header(USER_AGENT, user_agent.iter().collect::<String>()).send().await?.json().await?)
}

/// Call GitHub API to get an estimate size of a GitHub in bytes, or None if the API call fails
///
/// # Arguments
///
/// * `client`: The client to use to make the request
/// * `repo_full_name` The owner/name of the repo, e.x. "RLBot/RLBotPack"
async fn get_repo_size(client: &Client, repo_full_name: &str) -> Result<u64, Box<dyn Error>> {
    let data = get_json_from_url(client, &format!("https://api.github.com/repos/{}", repo_full_name)).await?;
    match data["size"].as_u64() {
        Some(size) => Ok(size * 1000),
        None => Err(Box::new(IoError::new(IoErrorKind::Other, "Failed to get repository size"))),
    }
}

/// An update packet that the GUI understands
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProgressBarUpdate {
    pub percent: f32,
    pub status: String,
}

impl ProgressBarUpdate {
    pub const fn new(percent: f32, status: String) -> Self {
        Self { percent, status }
    }
}

/// Downloads and extracts a zip file, passing updates to the GUI and returning an error if something went wrong when downloading
///
/// Errors other than during the download process are printed to the console
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `client`: The client to use to make the request
/// * `download_url`: The URL to get the zip from
/// * `local_folder_path`: The path to the folder to extract the zip to
/// * `clobber`: Deletes `local_folder_path` if it already exists
/// * `repo_full_name`: The owner/name of the repo, e.x. "RLBot/RLBotPack"
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
            if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, "Downloading zip...".to_owned())) {
                ccprintlne(window, format!("Error when updating progress bar: {}", e));
            }
            last_update = Instant::now();
        }
    }

    if clobber && local_folder_path.exists() {
        if let Err(e) = dir::remove(local_folder_path) {
            ccprintlne(window, format!("Error when removing existing folder: {}", e));
        }
    }

    if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(100., "Extracting zip...".to_owned())) {
        ccprintlne(window, format!("Error when updating progress bar: {}", e));
    }

    if let Err(e) = zip_extract_fixed::extract(window, Cursor::new(bytes), local_folder_path, false, true) {
        ccprintlne(window, format!("Error when extracting zip: {}", e));
    }

    Ok(())
}

/// Handles downloading a Github repo
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `repo_owner`: The owner of the repo, e.x. "RLBot"
/// * `repo_name`: The name of the repo, e.x. "RLBotPack"
/// * `checkout_folder`: The folder to checkout the repo to
/// * `update_tag_settings`: Whether to update the incr tag in the GUI config
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
            Ok(release) => release["tag_name"].as_str().unwrap().to_owned(),
            Err(e) => {
                ccprintlne(window, e.to_string());
                return BotpackStatus::Success("Downloaded the bot pack, but failed to get the latest release tag.".to_owned());
            }
        };

        let config_path = get_config_path();
        let mut config = load_gui_config(window);

        config.set("bot_folder_settings", "incr", Some(latest_release_tag_name));

        if let Err(e) = config.write(config_path) {
            ccprintlne(window, e.to_string());
            return BotpackStatus::Success("Downloaded the bot pack, but failed to write GUI's config.".to_owned());
        }
    }

    match status {
        Ok(_) => BotpackStatus::Success("Downloaded the bot pack!".to_owned()),
        Err(e) => {
            ccprintlne(window, e.to_string());
            BotpackStatus::Skipped("Failed to download the bot pack...".to_owned())
        }
    }
}

/// Load the GUI config and check the get the current version number of the botpack
pub fn get_current_tag_name() -> Option<u32> {
    load_cfg(get_config_path())
        .ok()?
        .get("bot_folder_settings", "incr")?
        .replace("incr-", "")
        .parse::<u32>()
        .ok()
}

/// Gets the corresponding incremental zip to a version number
///
/// # Arguments
///
/// `repo_full_name`: The owner/name of the repo, e.x. "RLBot/RLBotPack"
/// `tag`: The tag number, e.x. `103`
fn get_url_from_tag(repo_full_name: &str, tag: u32) -> String {
    format!("https://github.com/{}/releases/download/incr-{}/incremental.zip", repo_full_name, tag)
}

/// Check if the botpack is up to date
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `repo_full_name`: The owner/name of the repo, e.x. "RLBot/RLBotPack"
pub async fn is_botpack_up_to_date(window: &Window, repo_full_name: &str) -> bool {
    let current_tag_name = match get_current_tag_name() {
        Some(tag) => tag,
        None => return true,
    };

    let latest_release_tag = match get_json_from_url(&Client::new(), &format!("https://api.github.com/repos/{}/releases/latest", repo_full_name)).await {
        Ok(release) => release["tag_name"].as_str().unwrap().replace("incr-", "").parse::<u32>().unwrap(),
        Err(e) => {
            ccprintlne(window, format!("{}", e));
            return true;
        }
    };

    latest_release_tag == current_tag_name
}

/// Handles updating the botpack
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `repo_owner`: The owner of the repo, e.x. "RLBot"
/// * `repo_name`: The name of the repo, e.x. "RLBotPack"
/// * `checkout_folder`: The folder to checkout the repo to
pub async fn update_bot_pack(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str) -> BotpackStatus {
    let repo_full_name = format!("{}/{}", repo_owner, repo_name);

    let current_tag_name = match get_current_tag_name() {
        Some(tag) => tag,
        None => return BotpackStatus::RequiresFullDownload,
    };

    let latest_release_tag = match get_json_from_url(&Client::new(), &format!("https://api.github.com/repos/{}/releases/latest", repo_full_name)).await {
        Ok(release) => release["tag_name"].as_str().unwrap().replace("incr-", "").parse::<u32>().unwrap(),
        Err(e) => {
            ccprintlne(window, format!("{}", e));
            return BotpackStatus::Skipped("Failed to get the latest release tag.".to_owned());
        }
    };

    if latest_release_tag == current_tag_name {
        ccprintln(window, "The botpack is already up-to-date!".to_owned());
        return BotpackStatus::Skipped("The botpack is already up-to-date!".to_owned());
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

    let config_path = get_config_path();
    let mut config = load_gui_config(window);

    let tag_deleted_files_path = local_folder_path.join(".deleted");

    let mut handles = Vec::new();

    for tag in current_tag_name + 1..latest_release_tag + 1 {
        let url = get_url_from_tag(&repo_full_name, tag);

        handles.push(task::spawn(async move { Client::new().get(url).send().await }));
    }

    let mut tag = current_tag_name + 1;

    for handle in handles {
        ccprintln(window, format!("Patching in update incr-{}", tag));

        let progress = (tag - current_tag_name) as f32 / total_patches as f32 * 100.;
        if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, format!("Downloading patch incr-{}...", tag))) {
            ccprintlne(window, format!("Error when updating progress bar: {}", e));
        }

        let resp = match handle.await {
            Ok(resp) => resp,
            Err(e) => {
                ccprintlne(window, format!("Failed to await handle: {}", e));
                break;
            }
        };

        let download = match resp {
            Ok(download) => download,
            Err(e) => {
                ccprintlne(window, format!("Failed to download upgrade zip: {}", e));
                break;
            }
        };

        let progress = progress + 1. / (total_patches as f32 * 2.) * 100.;
        if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(progress, format!("Applying patch incr-{}...", tag))) {
            ccprintlne(window, format!("Error when updating progress bar: {}", e));
        }

        let bytes = match download.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                ccprintlne(window, format!("Failed to download upgrade zip: {}", e));
                break;
            }
        };

        if let Err(e) = zip_extract_fixed::extract(window, Cursor::new(&bytes), local_folder_path.as_path(), false, true) {
            ccprintlne(window, format!("Failed to extract upgrade zip: {}", e));
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
                            ccprintlne(window, format!("Failed to delete {}: {}", file_name.display(), e));
                            last_ok = false;
                        } else {
                            let text = format!("Deleted {}", file_name.display());
                            if last_ok {
                                ccprintlnr(window, text);
                            } else {
                                ccprintln(window, text);
                            }
                            last_ok = true;
                            count += 1;
                        }
                    }
                }

                let text = format!("Deleted {} files", count);
                if last_ok {
                    ccprintlnr(window, text);
                } else {
                    ccprintln(window, text);
                }
            }
            Err(e) => {
                ccprintlne(window, format!("Failed to open .deleted file: {}", e));
                break;
            }
        }

        config.set("bot_folder_settings", "incr", Some(format!("incr-{}", tag)));

        if let Err(e) = config.write(&config_path) {
            ccprintlne(window, e.to_string());
        }

        tag += 1;

        if tag_deleted_files_path.exists() {
            if let Err(e) = remove_file(&tag_deleted_files_path) {
                ccprintlne(window, format!("Failed to delete {}: {}", tag_deleted_files_path.display(), e));
            }
        }
    }

    if let Err(e) = remove_empty_folders(window, local_folder_path) {
        ccprintlne(window, format!("Failed to remove empty folders: {}", e));
    }

    if tag - 1 == latest_release_tag {
        BotpackStatus::Success("Updated the botpack!".to_owned())
    } else {
        BotpackStatus::Skipped("Failed to update the botpack...".to_owned())
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
    pub fn get_map_index(&self, window: &Window) -> Option<serde_json::Value> {
        let index_path = self.full_path.join("index.json");

        if index_path.exists() {
            let mut file = match File::open(index_path) {
                Ok(file) => file,
                Err(e) => {
                    ccprintlne(window, format!("Failed to open index.json: {}", e));
                    return None;
                }
            };

            let mut contents = String::new();

            if let Err(e) = file.read_to_string(&mut contents) {
                ccprintlne(window, format!("Failed to read index.json: {}", e));
                return None;
            }

            match serde_json::from_str(&contents) {
                Ok(json) => Some(json),
                Err(e) => {
                    ccprintlne(window, format!("Failed to parse index.json: {}", e));
                    None
                }
            }
        } else {
            None
        }
    }

    /// Compares the old_index with current index and for any
    /// maps that have updated the revision, we grab them
    /// from the latest revision
    pub async fn needs_update(&self, window: &Window) -> BotpackStatus {
        let index = match self.get_map_index(window) {
            Some(index) => index,
            None => return BotpackStatus::RequiresFullDownload,
        };

        let revision = index["revision"].as_u64().unwrap();

        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", self.repo_owner, self.repo_name);

        let latest_release = match get_json_from_url(&self.client, &url).await {
            Ok(latest_release) => latest_release,
            Err(e) => {
                ccprintlne(window, format!("Failed to get latest release: {}", e));
                return BotpackStatus::Skipped("Failed to get latest release".to_owned());
            }
        };

        let latest_revision = latest_release["tag_name"].as_str().unwrap()[1..].parse::<u64>().unwrap();

        if latest_revision > revision {
            BotpackStatus::RequiresFullDownload
        } else {
            ccprintln(window, "Map pack is already up-to-date!".to_owned());
            BotpackStatus::Skipped("Map pack is already up-to-date!".to_owned())
        }
    }

    fn extract_maps_from_index(index: serde_json::Value) -> HashMap<String, u64> {
        index["maps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|map| (map["path"].as_str().unwrap().to_owned(), map["revision"].as_u64().unwrap()))
            .collect::<HashMap<String, u64>>()
    }

    /// Compares the old_index with current index and for any
    /// maps that have updated the revision, we grab them
    /// from the latest revision
    pub async fn hydrate_map_pack(&self, window: &Window, old_index: Option<serde_json::Value>) {
        let new_maps = match self.get_map_index(window) {
            Some(index) => Self::extract_maps_from_index(index),
            None => {
                ccprintlne(window, "Failed to get index.json".to_owned());
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
                to_fetch.insert(path.to_owned());
            }
        }

        if to_fetch.is_empty() {
            return;
        }

        let mut filename_to_path = HashMap::new();
        for path in to_fetch.iter() {
            let filename = Path::new(path).file_name().unwrap().to_string_lossy();
            filename_to_path.insert(filename.to_string(), path.to_owned());
        }

        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", self.repo_owner, self.repo_name);

        let latest_release = match get_json_from_url(&self.client, &url).await {
            Ok(latest_release) => latest_release,
            Err(e) => {
                ccprintlne(window, format!("Failed to get latest release: {}", e));
                return;
            }
        };

        for asset in latest_release["assets"].as_array().unwrap() {
            let asset_name = asset["name"].as_str().unwrap();
            if let Err(e) = self.download_asset(window, asset, asset_name, &filename_to_path, &self.full_path).await {
                ccprintlne(window, format!("Failed to download asset {}: {}", asset_name, e));
            }
        }
    }

    async fn download_asset<T: AsRef<Path>>(
        &self,
        window: &Window,
        asset: &serde_json::Value,
        asset_name: &str,
        filename_to_path: &HashMap<String, String>,
        full_path: T,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(local_path) = filename_to_path.get(asset_name) {
            let target_path = full_path.as_ref().join(local_path);
            ccprintln(window, format!("Will fetch updated map {}", asset_name));

            let url = asset["browser_download_url"].as_str().unwrap();
            let resp = self.client.get(url).send().await?.bytes().await?;
            let mut file = File::create(target_path)?;
            file.write_all(&resp)?;
        }

        Ok(())
    }
}
