use std::{
    error::Error,
    fs::{read_dir, remove_dir},
    io::Cursor,
    path::Path,
};

use tauri::Window;

const RELEASE_TAG: &str = "latest_botpack_release_tag";
const FOLDER_SUFFIX: &str = "master";

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

async fn get_json_from_url(url: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(reqwest::get(url).await?.json::<serde_json::Value>().await?)
}

/// Returns Size of the repository in bytes, or None if the API call fails.
///
/// Call GitHub API to get an estimate size of a GitHub repository.
///
/// * `repo_full_name` Full name of a repository. Example: 'RLBot/RLBotPack'
async fn get_repo_size(repo_full_name: &str) -> Result<u64, Box<dyn Error>> {
    let data = get_json_from_url(&format!("https://api.github.com/repos/{}", repo_full_name)).await?;
    Ok(data["size"].as_u64().unwrap() * 1000)
}

async fn download_and_extract_zip<T: AsRef<Path>>(download_url: T, local_folder_path: T, local_folder_subname: T, clobber: bool) -> BotpackStatus {
    // download and extract the zip
    let local_folder_path = local_folder_path.as_ref();

    if clobber && local_folder_path.exists() {
        remove_dir(local_folder_path).unwrap();
    }

    match reqwest::get("https://github.com/RLBot/RLBotPythonExample/archive/master.zip").await {
        Ok(res) => {
            zip_extract::extract(Cursor::new(&res.bytes().await.unwrap()), local_folder_path, false).unwrap();
            BotpackStatus::Success
        }
        Err(_) => BotpackStatus::Skipped,
    }
}

// class RepoDownloader:
//     """
//     Downloads the given repo while updating the progress bar and status text.
//     """

//     PROGRESSBAR_UPDATE_INTERVAL = 0.1  # How often to update the progress bar (seconds)

//     def __init__(self):
//         self.status = ''
//         self.total_progress = 0

//         self.estimated_zip_size = 0
//         self.downloaded_bytes = 0
//         self.last_progressbar_update_time = 0

//     def update_progressbar_and_status(self):
//         # it's not necessary to update on every callback, so update
//         # only when some amount of time has passed
//         now = time.time()
//         if now > self.last_progressbar_update_time + self.PROGRESSBAR_UPDATE_INTERVAL:
//             self.last_progressbar_update_time = now

//             total_progress_percent = int(self.total_progress * 100)
//             status = f'{self.status} ({total_progress_percent}%)'

//             eel.updateDownloadProgress(total_progress_percent, status)

//     def zip_download_callback(self, block_count, block_size, _):
//         self.downloaded_bytes += block_size
//         self.total_progress = min(self.downloaded_bytes / self.estimated_zip_size, 1.0)
//         self.update_progressbar_and_status()

//     def unzip_callback(self):
//         eel.updateDownloadProgress(100, 'Extracting ZIP file')

//     def download(self, repo_owner: str, repo_name: str, checkout_folder: Path, update_tag_setting=True):
//         repo_full_name = repo_owner + '/' + repo_name
//         folder_suffix = FOLDER_SUFFIX

//         self.status = f'Downloading {repo_full_name}-{folder_suffix}'
//         print(self.status)
//         self.total_progress = 0

//         # Unfortunately we can't know the size of the zip file before downloading it,
//         # so we have to get the size from the GitHub API.
//         self.estimated_zip_size = get_repo_size(repo_full_name)
//         if self.estimated_zip_size:
//             # Github's compression ratio for the botpack is around 75%
//             self.estimated_zip_size *= 0.75

//         # If we fail to get the repo size, set it to a fallback value,
//         # so the progress bar will show at least some progress.
//         # Let's assume the zip file is around 60 MB.
//         else:
//             self.estimated_zip_size = 60_000_000

//         try:
//             latest_release = get_json_from_url(f"https://api.github.com/repos/{repo_owner}/{repo_name}/releases/latest")
//         except Exception as err:
//             print(err)
//             return BotpackStatus.SKIPPED

//         success = download_and_extract_zip(download_url=latest_release['zipball_url'],
//                                  local_folder_path=checkout_folder,
//                                  local_subfolder_name=f"{repo_name}-{folder_suffix}",
//                                  clobber=True,
//                                  progress_callback=self.zip_download_callback,
//                                  unzip_callback=self.unzip_callback)

//         if success is BotpackStatus.SUCCESS and update_tag_setting:
//             settings = load_settings()
//             settings.setValue(RELEASE_TAG, latest_release["tag_name"])

//         return success

pub async fn download_repo(window: &Window, repo_owner: &str, repo_name: &str, checkout_folder: &str, update_tag_settings: bool) -> BotpackStatus {
    // let repo_full_name = format!("{}/{}", repo_owner, repo_name);

    let latest_release = get_json_from_url(&format!("https://api.github.com/repos/{}/{}/releases/latest", repo_owner, repo_name))
        .await
        .unwrap();
    let status = download_and_extract_zip(
        latest_release["zipball_url"].as_str().unwrap(),
        checkout_folder,
        &format!("{}-{}", repo_name, FOLDER_SUFFIX),
        true,
    )
    .await;

    status
}
