use super::{
    cfg_helper::{load_cfg, save_cfg},
    zip_extract_fixed,
};
use crate::{ccprintln, commands::UPDATE_DOWNLOAD_PROGRESS_SIGNAL, emit_text, get_config_path, load_gui_config};
use fs_extra::dir;
use futures_util::StreamExt;
use rand::Rng;
use reqwest::{header::USER_AGENT, Client, IntoUrl};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::{read_dir, remove_dir, remove_file, File},
    io::{BufRead, BufReader, Cursor, Error as IoError, ErrorKind as IoErrorKind, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
    time::Instant,
};
use tauri::Window;
use tokio::{fs as async_fs, task};

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
        ccprintln!(window, "Removed empty folder: {}", dir.display());
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
    // get random string 8-character string
    let user_agent: String = rand::thread_rng().gen::<[char; 8]>().iter().collect();
    Ok(client.get(url).header(USER_AGENT, user_agent).send().await?.json().await?)
}

/// Call GitHub API to get an estimate size of a GitHub in bytes, or None if the API call fails
///
/// # Arguments
///
/// * `client`: The client to use to make the request
/// * `repo_full_name` The owner/name of the repo, e.x. "RLBot/RLBotPack"
async fn get_repo_size(client: &Client, repo_full_name: &str) -> Result<u64, Box<dyn Error>> {
    let data = get_json_from_url(client, &format!("https://api.github.com/repos/{repo_full_name}")).await?;
    let json_size = data.get("size").ok_or("Failed to get size from GitHub API")?;
    let Some(size) = json_size.as_u64() else {
        return Err(Box::new(IoError::new(IoErrorKind::Other, "Failed to get repository size")));
    };

    Ok(size * 1000)
}

/// An update packet that the GUI understands
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProgressBarUpdate {
    pub percent: f64,
    pub status: String,
}

impl ProgressBarUpdate {
    pub const fn new(percent: f64, status: String) -> Self {
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

    let uncompresed_size = get_repo_size(client, repo_full_name).await.unwrap_or(170_000_000);
    let real_size_estimate = uncompresed_size * 62 / 100;
    let mut stream = res.bytes_stream();
    let mut bytes = Vec::with_capacity(real_size_estimate as usize);
    let mut last_update = Instant::now();
    let real_size_estimate = real_size_estimate as f64;

    while let Some(new_bytes) = stream.next().await {
        // put the new bytes into bytes
        bytes.extend_from_slice(&new_bytes?);

        if last_update.elapsed().as_secs_f32() >= 0.1 {
            let progress = bytes.len() as f64 / real_size_estimate * 100.0;
            if let Err(e) = window.emit(UPDATE_DOWNLOAD_PROGRESS_SIGNAL, ProgressBarUpdate::new(progress, "Downloading zip...".to_owned())) {
                ccprintln!(window, "Error when updating progress bar: {e}");
            }
            last_update = Instant::now();
        }
    }

    if clobber && local_folder_path.exists() {
        if let Err(e) = dir::remove(local_folder_path) {
            ccprintln!(window, "Error when removing existing folder: {e}");
        }
    }

    if let Err(e) = window.emit(UPDATE_DOWNLOAD_PROGRESS_SIGNAL, ProgressBarUpdate::new(100., "Extracting zip...".to_owned())) {
        ccprintln!(window, "Error when updating progress bar: {e}");
    }

    if let Err(e) = zip_extract_fixed::extract(window, Cursor::new(bytes), local_folder_path, false, true) {
        ccprintln!(window, "Error when extracting zip: {e}");
    }

    Ok(())
}

/// Handles downloading a Github repo
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `repo_owner`: The owner of the repo, e.x. `"RLBot"`
/// * `repo_name`: The name of the repo, e.x. `"RLBotPack"`
/// * `checkout_folder`: The folder to checkout the repo to
/// * `update_tag_settings`: Whether to update the incr tag in the GUI config
pub async fn download_repo(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str, update_tag_settings: bool) -> BotpackStatus {
    let client = Client::new();
    let repo_full_name = format!("{repo_owner}/{repo_name}");

    if let Err(e) = download_and_extract_repo_zip(
        window,
        &client,
        &format!("https://github.com/{repo_full_name}/archive/refs/heads/master.zip"),
        checkout_folder,
        true,
        &repo_full_name,
    )
    .await
    {
        ccprintln(window, e.to_string());
        return BotpackStatus::Skipped("Failed to download the bot pack...".to_owned());
    };

    if update_tag_settings {
        let latest_release_tag_name = match get_json_from_url(&client, &format!("https://api.github.com/repos/{repo_full_name}/releases/latest")).await {
            Ok(release) => release["tag_name"].as_str().unwrap_or_default().to_owned(),
            Err(e) => {
                ccprintln(window, e.to_string());
                return BotpackStatus::Success("Downloaded the bot pack, but failed to get the latest release tag.".to_owned());
            }
        };

        let config_path = get_config_path();
        let mut config = load_gui_config(window).await;

        config.set("bot_folder_settings", "incr", Some(latest_release_tag_name));

        if let Err(e) = save_cfg(&config, config_path).await {
            ccprintln(window, e.to_string());
            return BotpackStatus::Success("Downloaded the bot pack, but failed to write GUI's config.".to_owned());
        }
    }

    BotpackStatus::Success("Downloaded the bot pack!".to_owned())
}

/// Load the GUI config and check the get the current version number of the botpack
pub async fn get_current_tag_name() -> Option<u32> {
    load_cfg(get_config_path())
        .await
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
    format!("https://github.com/{repo_full_name}/releases/download/incr-{tag}/incremental.zip")
}

/// Finds what the tag is on the latest release in a repo
async fn get_latest_release_tag(repo_full_name: &str) -> Result<u32, String> {
    get_json_from_url(&Client::new(), &format!("https://api.github.com/repos/{repo_full_name}/releases/latest"))
        .await
        .map_err(|e| e.to_string())
        .and_then(|release| {
            release
                .get("tag_name")
                .ok_or_else(|| "No key 'tag_name' found in json".to_string())
                .and_then(|json_tag| json_tag.as_str().ok_or_else(|| "Couldn't convert tag_name to string".to_string()))
                .and_then(|tag_name| tag_name.replace("incr-", "").parse::<u32>().map_err(|e| e.to_string()))
        })
}

/// Check if the botpack is up to date
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `repo_full_name`: The owner/name of the repo, e.x. "RLBot/RLBotPack"
pub async fn is_botpack_up_to_date(window: &Window, repo_full_name: &str) -> bool {
    let Some(current_tag_name) = get_current_tag_name().await else {
        return true;
    };

    match get_latest_release_tag(repo_full_name).await {
        Ok(latest_release_tag) => latest_release_tag == current_tag_name,
        Err(e) => {
            ccprintln(window, e);
            true
        }
    }
}

/// Handles updating the botpack
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `repo_owner`: The owner of the repo, e.x. `"RLBot"`
/// * `repo_name`: The name of the repo, e.x. `"RLBotPack"`
/// * `checkout_folder`: The folder to checkout the repo to
pub async fn update_bot_pack(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str) -> BotpackStatus {
    let repo_full_name = format!("{repo_owner}/{repo_name}");

    let Some(current_tag_name) = get_current_tag_name().await else {
        return BotpackStatus::RequiresFullDownload;
    };

    let latest_release_tag = match get_latest_release_tag(&repo_full_name).await {
        Ok(value) => value,
        Err(e) => {
            ccprintln(window, e);
            return BotpackStatus::Skipped("Failed to get the latest release tag.".to_owned());
        }
    };

    if latest_release_tag == current_tag_name {
        ccprintln(window, "The botpack is already up-to-date!");
        return BotpackStatus::Skipped("The botpack is already up-to-date!".to_owned());
    }

    let total_patches = latest_release_tag - current_tag_name;

    if total_patches > 50 {
        return BotpackStatus::RequiresFullDownload;
    }

    let master_folder = format!("{repo_name}-{FOLDER_SUFFIX}");
    let local_folder_path = Path::new(checkout_folder).join(master_folder);

    if !local_folder_path.exists() {
        return BotpackStatus::RequiresFullDownload;
    }

    let config_path = get_config_path();
    let mut config = load_gui_config(window).await;

    let tag_deleted_files_path = local_folder_path.join(".deleted");

    let handles = ((current_tag_name + 1)..=latest_release_tag)
        .map(|tag| get_url_from_tag(&repo_full_name, tag))
        .map(|url| task::spawn(async move { Client::new().get(url).send().await }))
        .collect::<Vec<_>>();

    let mut tag = current_tag_name + 1;
    let total_patches = f64::from(total_patches);

    for handle in handles {
        let patch_status = format!("Patching in update incr-{tag}");
        ccprintln(window, &patch_status);

        let progress = f64::from(tag - current_tag_name) / total_patches * 100.;
        if let Err(e) = window.emit(UPDATE_DOWNLOAD_PROGRESS_SIGNAL, ProgressBarUpdate::new(progress, patch_status)) {
            ccprintln!(window, "Error when updating progress bar: {e}");
        }

        let resp = match handle.await {
            Ok(resp) => resp,
            Err(e) => {
                ccprintln!(window, "Error awaiting handle: {e}");
                break;
            }
        };

        let progress = progress + 1. / (total_patches * 2.) * 100.;
        if let Err(e) = window.emit(UPDATE_DOWNLOAD_PROGRESS_SIGNAL, ProgressBarUpdate::new(progress, format!("Applying patch incr-{tag}..."))) {
            ccprintln!(window, "Error when updating progress bar: {}", e);
        }

        if let ControlFlow::Break(_) = apply_patch(resp, window, &local_folder_path, &tag_deleted_files_path).await {
            break;
        }

        config.set("bot_folder_settings", "incr", Some(format!("incr-{tag}")));

        if let Err(e) = save_cfg(&config, &config_path).await {
            ccprintln(window, e.to_string());
        }

        tag += 1;

        if tag_deleted_files_path.exists() {
            if let Err(e) = remove_file(&tag_deleted_files_path) {
                ccprintln!(window, "Error deleting {}: {e}", tag_deleted_files_path.display());
            }
        }
    }

    if let Err(e) = remove_empty_folders(window, local_folder_path) {
        ccprintln!(window, "Error removing empty folders: {e}");
    }

    if tag - 1 == latest_release_tag {
        BotpackStatus::Success("Updated the botpack!".to_owned())
    } else {
        BotpackStatus::Skipped("Failed to update the botpack...".to_owned())
    }
}

/// Applies a single patch to the botpack
///
/// # Arguments
///
/// * `resp`: The response from the HTTP request
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `local_folder_path`: The path to the local folder containing the botpack
/// * `tag_deleted_files_path`: The path to the file containing the deleted files for the patch
async fn apply_patch(resp: Result<reqwest::Response, reqwest::Error>, window: &Window, local_folder_path: &Path, tag_deleted_files_path: &Path) -> ControlFlow<()> {
    let download = match resp {
        Ok(download) => download,
        Err(e) => {
            ccprintln!(window, "Error downloading upgrade zip: {e}");
            return ControlFlow::Break(());
        }
    };

    let bytes = match download.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            ccprintln!(window, "Error downloading upgrade zip: {e}");
            return ControlFlow::Break(());
        }
    };

    if let Err(e) = zip_extract_fixed::extract(window, Cursor::new(&bytes), local_folder_path, false, true) {
        ccprintln!(window, "Error extracting upgrade zip: {e}");
        return ControlFlow::Break(());
    }

    let file = match File::open(tag_deleted_files_path) {
        Ok(file) => file,
        Err(e) => {
            ccprintln!(window, "Error opening .deleted file: {e}");
            return ControlFlow::Break(());
        }
    };

    let mut last_ok = false;
    let mut count = 0;
    for line in BufReader::new(file).lines().flatten() {
        let line = line.replace(['\0', '\r'], "");
        if !line.is_empty() {
            let file_name = local_folder_path.join(line);
            if let Err(e) = remove_file(&file_name) {
                ccprintln!(window, "Error deleting {}: {e}", file_name.display());
                last_ok = false;
            } else {
                emit_text(window, format!("Deleted {}", file_name.display()), last_ok);
                last_ok = true;
                count += 1;
            }
        }
    }

    emit_text(window, format!("Deleted {count} files"), last_ok);
    ControlFlow::Continue(())
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
            full_path: location.as_ref().join(format!("{repo_name}-main")),
            repo_owner,
            repo_name,
            client: Client::new(),
        }
    }

    /// For a map pack, gets you the index.json data
    pub async fn get_map_index(&self, window: &Window) -> Option<serde_json::Value> {
        let index_path = self.full_path.join("index.json");

        if index_path.exists() {
            let contents = match async_fs::read_to_string(index_path).await {
                Ok(contents) => contents,
                Err(e) => {
                    ccprintln!(window, "Error reading index.json: {e}");
                    return None;
                }
            };

            match serde_json::from_str(&contents) {
                Ok(json) => Some(json),
                Err(e) => {
                    ccprintln!(window, "Error parseing index.json: {e}");
                    None
                }
            }
        } else {
            None
        }
    }

    /// Compares the `old_index` with current index and for any
    /// maps that have updated the revision, we grab them
    /// from the latest revision
    pub async fn needs_update(&self, window: &Window) -> BotpackStatus {
        let Some(index) = self.get_map_index(window).await else {
            return BotpackStatus::RequiresFullDownload;
        };
        let revision = index["revision"].as_u64().unwrap();
        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", self.repo_owner, self.repo_name);

        let latest_release = match get_json_from_url(&self.client, &url).await {
            Ok(latest_release) => latest_release,
            Err(e) => {
                ccprintln!(window, "Error getting latest release: {e}");
                return BotpackStatus::Skipped("Failed to get latest release".to_owned());
            }
        };

        let latest_revision = latest_release["tag_name"].as_str().unwrap()[1..].parse::<u64>().unwrap();

        if latest_revision > revision {
            BotpackStatus::RequiresFullDownload
        } else {
            ccprintln(window, "Map pack is already up-to-date!");
            BotpackStatus::Skipped("Map pack is already up-to-date!".to_owned())
        }
    }

    fn extract_maps_from_index(index: &serde_json::Value) -> HashMap<String, u64> {
        index["maps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|map| (map["path"].as_str().unwrap().to_owned(), map["revision"].as_u64().unwrap()))
            .collect::<HashMap<String, u64>>()
    }

    /// Compares the `old_index` with current index and for any
    /// maps that have updated the revision, we grab them
    /// from the latest revision
    pub async fn hydrate_map_pack(&self, window: &Window, old_index: Option<serde_json::Value>) {
        let Some(index) = self.get_map_index(window).await else {
            ccprintln(window, "Error getting index.json");
            return;
        };

        let new_maps = Self::extract_maps_from_index(&index);

        let old_maps = match old_index {
            Some(index) => Self::extract_maps_from_index(&index),
            None => HashMap::new(),
        };

        let mut to_fetch = HashSet::new();
        for (path, revision) in &new_maps {
            if !old_maps.contains_key(path) || old_maps[path] < *revision {
                to_fetch.insert(path.clone());
            }
        }

        if to_fetch.is_empty() {
            return;
        }

        let mut filename_to_path = HashMap::new();
        for path in &to_fetch {
            let filename = Path::new(path).file_name().unwrap().to_string_lossy();
            filename_to_path.insert(filename.to_string(), path.clone());
        }

        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", self.repo_owner, self.repo_name);

        let latest_release = match get_json_from_url(&self.client, &url).await {
            Ok(latest_release) => latest_release,
            Err(e) => {
                ccprintln!(window, "Error getting latest release: {e}");
                return;
            }
        };

        for asset in latest_release["assets"].as_array().unwrap() {
            let asset_name = asset["name"].as_str().unwrap();
            if let Err(e) = self.download_asset(window, asset, asset_name, &filename_to_path, &self.full_path).await {
                ccprintln!(window, "Error downloading asset {asset_name}: {e}");
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
            ccprintln!(window, "Will fetch updated map {asset_name}");

            let url = asset["browser_download_url"].as_str().unwrap();
            let resp = self.client.get(url).send().await?.bytes().await?;
            let mut file = File::create(target_path)?;
            file.write_all(&resp)?;
        }

        Ok(())
    }
}
