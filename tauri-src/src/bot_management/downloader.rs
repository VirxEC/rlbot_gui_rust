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

use futures_util::{StreamExt, stream};

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

async fn download_and_extract_repo_zip<'a, T: IntoUrl, J: AsRef<Path>>(
    window: &'a Window,
    client: &'a Client,
    download_url: T,
    local_folder_path: J,
    clobber: bool,
    repo_full_name: &'a str,
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
                ccprintln(format!("Error when updating progress bar: {}", e));
            }
            last_update = Instant::now();
        }
    }

    if clobber && local_folder_path.exists() {
        fs_extra::dir::remove(local_folder_path).unwrap();
    }

    if let Err(e) = window.emit("update-download-progress", ProgressBarUpdate::new(100., "Extracting zip...".to_string())) {
        ccprintln(format!("Error when updating progress bar: {}", e));
    }

    zip_extract::extract(Cursor::new(bytes), local_folder_path, false).unwrap();
    Ok(())
}

pub async fn download_repo(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str, update_tag_settings: bool) -> BotpackStatus {
    let client = reqwest::Client::new();
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

    match status {
        Ok(_) => BotpackStatus::Success,
        Err(e) => {
            ccprintln(e.to_string());
            BotpackStatus::Skipped
        }
    }
}

// def update(self, repo_owner: str, repo_name: str, checkout_folder: Path):
//     repo_full_name = repo_owner + '/' + repo_name
//     repo_url = 'https://github.com/' + repo_full_name
//     master_folder = repo_name + "-" + FOLDER_SUFFIX

//     settings = load_settings()
//     local_release_tag = settings.value(RELEASE_TAG, type=str)

//     try:
//         latest_release = get_json_from_url(f"https://api.github.com/repos/{repo_owner}/{repo_name}/releases/latest")
//     except Exception as err:
//         print(err)
//         return BotpackStatus.REQUIRES_FULL_DOWNLOAD

//     # If the botpack is missing, just download the whole botpack
//     if local_release_tag == "" or not os.path.exists(os.path.join(checkout_folder, master_folder)):
//         return BotpackStatus.REQUIRES_FULL_DOWNLOAD

//     if local_release_tag == latest_release["tag_name"]:
//         print("The botpack is already up-to-date! Redownloading just in case.")
//         return BotpackStatus.REQUIRES_FULL_DOWNLOAD

//     releases_to_download = list(range(int(local_release_tag.replace("incr-", "")) + 1, int(latest_release["tag_name"].replace("incr-", "")) + 1))

//     # If there are too many patches to be applied at once, don't bother and instead do a full redownload of the bot pack. Each patch has a certain
//     # amount of overhead so at some point it becomes faster to do a full download. We also do not want to spam github with too many download requests.
//     if len(releases_to_download) > 50:
//         return BotpackStatus.REQUIRES_FULL_DOWNLOAD

//     local_folder_path = Path(os.path.join(checkout_folder, master_folder))

//     self.total_steps = len(releases_to_download)
//     with tempfile.TemporaryDirectory() as tmpdir:
//         # Spawn up to 15 download threads, we want to download the updates at a fast speed without saturating the users network connection.
//         # These threads only serve to initiate the download and mostly sit idle.
//         with mp.Pool(min(15, len(releases_to_download))) as p:
//             # It's very important that patches are applied in order
//             # This is why we use imap and not imap_unordered
//             # we want simultaneous downloads, but applying patches out of order would be a very bad idea
//             for tag in p.imap(partial(self.download_single, tmpdir, repo_url), releases_to_download):
//                 if tag is False:
//                     print("Failed to complete botpack upgrade")
//                     return BotpackStatus.SKIPPED

//                 # apply incremental patch
//                 print(f"Applying patch incr-{tag}")
//                 self.update_progressbar_and_status(f"Applying patch {tag}")
//                 downloaded_zip_path = os.path.join(tmpdir, f"downloaded-{tag}.zip")

//                 with zipfile.ZipFile(downloaded_zip_path, 'r') as zip_ref:
//                     zip_ref.extractall(local_folder_path)

//                     # Zip was made on Windows using Powershell
//                     # Files will all be called something like "RLBotPack\Necto\bot.cfg" instead of being in their folders
//                     # All we need to do is loop through the files and rename them
//                     if platform.system() != 'Windows':
//                         print("Not on Windows, placing files in their folders")
//                         members = zip_ref.namelist()

//                         for zipinfo in members:
//                             if zipinfo.count("\\") == 0:
//                                 continue

//                             old_path = local_folder_path / zipinfo
//                             new_path = local_folder_path / zipinfo.replace("\\", "/")

//                             if zipinfo[-1] == "\\":
//                                 if not os.path.exists(new_path):
//                                     os.makedirs(new_path)
//                                 os.remove(old_path)
//                                 continue

//                             if not os.path.isdir(new_path.parent):
//                                 os.makedirs(new_path.parent)

//                             os.rename(old_path, new_path)

//                 with open(local_folder_path / ".deleted", "r", encoding="utf-16") as deleted_ref:
//                     files = deleted_ref.readlines()

//                     for line in files:
//                         if line.replace("\n", "").strip() != "":
//                             file_name = local_folder_path / line.replace("\n", "")
//                             if os.path.isfile(file_name):
//                                 os.remove(file_name)

//                 # clean up .deleted
//                 os.remove(local_folder_path / ".deleted")

//                 # encase something goes wrong in the future, we can save our place between commit upgrades
//                 settings.setValue(RELEASE_TAG, f"incr-{tag}")
//                 self.current_step += 1

//     remove_empty_folders(local_folder_path)

//     self.update_progressbar_and_status(f"Done")
//     return BotpackStatus.SUCCESS

fn get_current_tag_name() -> Option<u32> {
    let config_path = get_config_path();
    let mut config = Ini::new();
    config.load(&config_path).ok()?;

    config.get("bot_folder_settings", "incr")?.replace("incr-", "").parse::<u32>().ok()
}

const CONCURRENT_REQUESTS: usize = 15;

pub async fn update_bot_pack(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str) -> BotpackStatus {
    let client = Client::new();
    let repo_full_name = format!("{}/{}", repo_owner, repo_name);

    let current_tag_name = match get_current_tag_name() {
        Some(tag) => tag,
        None => return BotpackStatus::RequiresFullDownload,
    };

    // let latest_release_tag_name = match get_json_from_url(&client, &format!("https://api.github.com/repos/{}/releases/latest", repo_full_name)).await {
    //     Ok(release) => release["tag_name"].as_str().unwrap().replace("incr-", "").parse::<u32>().unwrap(),
    //     Err(e) => {
    //         ccprintln(format!("{}", e));
    //         return BotpackStatus::Skipped;
    //     }
    // };
    let latest_release_tag_name = 75;

    if latest_release_tag_name == current_tag_name {
        ccprintln("The botpack is already up-to-date!".to_string());
        return BotpackStatus::Skipped;
    }

    let urls = (current_tag_name + 1..latest_release_tag_name + 1)
        .into_iter()
        .map(|tag| format!("https://github.com/{}/releases/download/incr-{}/incremental.zip", repo_full_name, tag))
        .collect::<Vec<String>>();
    dbg!(&urls);

    let bodies = stream::iter(urls)
        .enumerate()
        .map(|(i, url)| {
            let client = &client;
            async move {
                let resp = client.get(url).send().await?;
                match resp.bytes().await {
                    Ok(bytes) => Ok((i, bytes)),
                    Err(e) => Err(e),
                }
            }
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    bodies
        .for_each(|b| async {
            match b {
                Ok(b) => println!("{} got {} bytes", b.0, b.1.len()),
                Err(e) => eprintln!("God an error: {}", e),
            }
        })
        .await;

    BotpackStatus::Success
}
