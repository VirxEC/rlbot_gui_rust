use crate::{ccprintln, ccprintlnr};
use std::{
    fs,
    io::{copy, Read, Seek},
    path::{Path, PathBuf, StripPrefixError},
};
use tauri::Window;
use thiserror::Error;
use zip::{result::ZipError, ZipArchive};

// Code taken from https://github.com/MCOfficer/zip-extract
// License: MIT
// Code taken due to lack up updates, a few prominent bugs & a lack of eyes from the community (potential security flaw)
// As a result, the code has been patched and debugging as been better integrated into the GUI

/// The error type for the `extract` function.
#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("Invalid ZIP archive: {0}")]
    Zip(#[from] ZipError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Couldn't strip the top level ({top_level}) from {path}")]
    StripToplevel {
        top_level: PathBuf,
        path: PathBuf,
        #[source]
        error: StripPrefixError,
    },
}

/// Extract a zip file to a directory with GUI console prints
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `source`: The source zip file to extract
/// * `target_dir`: The target directory to extract the zip file to
/// * `toplevel`: If the top level directory to strip from the zip file (does nothing if there are multiple top level directories)
/// * `replace`: Whether or not files should be overwritten if they already exist in the target directory
pub fn extract<S: Read + Seek>(window: &Window, source: S, target_dir: &Path, strip_toplevel: bool, replace: bool) -> Result<(), ExtractError> {
    if !target_dir.exists() {
        fs::create_dir_all(target_dir)?;
    }

    let mut archive = ZipArchive::new(source)?;
    let num_files = archive.len();

    let do_strip_toplevel = strip_toplevel && has_toplevel(window, &mut archive)?;

    ccprintln!(window, "Extracting to {}", target_dir.to_string_lossy());
    ccprintln!(window);
    for i in 0..num_files {
        let mut item = archive.by_index(i)?;
        let Some(enclosed_name) = item.enclosed_name() else {
            continue;
        };
        let mut relative_path = if cfg!(windows) {
            enclosed_name.to_path_buf()
        } else {
            PathBuf::from(enclosed_name.to_string_lossy().replace('\\', "/"))
        };

        if do_strip_toplevel {
            let base = relative_path.components().take(1).fold(PathBuf::new(), |mut p, c| {
                p.push(c);
                p
            });
            relative_path = relative_path
                .strip_prefix(&base)
                .map_err(|error| ExtractError::StripToplevel {
                    top_level: base,
                    path: relative_path.clone(),
                    error,
                })?
                .to_path_buf();
        }

        if relative_path.to_string_lossy().is_empty() {
            // Top-level directory
            continue;
        }

        let outpath = target_dir.join(&relative_path);

        if item.is_dir() {
            ccprintlnr!(window, "Creating directory {} from {}", outpath.to_string_lossy(), relative_path.display());
            if !outpath.exists() {
                if let Err(e) = fs::create_dir_all(&outpath) {
                    ccprintln!(window, "Error creating directory {}: {e}", outpath.display());
                }
            }
            continue;
        }

        if outpath.exists() {
            if replace {
                fs::remove_file(&outpath)?;
            } else {
                continue;
            }
        } else if let Some(p) = outpath.parent() {
            if !p.exists() {
                if let Err(e) = fs::create_dir_all(p) {
                    ccprintln!(window, "Error creating directory {}: {e}", p.display());
                }
            }
        }

        ccprintlnr!(window, "({i}/{num_files}) Creating {} from {}", outpath.to_string_lossy(), relative_path.display());
        let mut outfile = fs::File::create(&outpath)?;
        copy(&mut item, &mut outfile)?;
    }

    ccprintlnr!(window, "Extracted {num_files} items");
    Ok(())
}

/// Check if the zip file has a top level directory
///
/// # Arguments
///
/// * `window`: A reference to the GUI, obtained from a `#[tauri::command]` function
/// * `archive`: The zip archive to check
fn has_toplevel<S: Read + Seek>(window: &Window, archive: &mut ZipArchive<S>) -> Result<bool, ZipError> {
    let mut toplevel_dir: Option<PathBuf> = None;
    if archive.len() < 2 {
        return Ok(false);
    }

    for i in 0..archive.len() {
        let file = archive.by_index(i)?.mangled_name();
        if let Some(toplevel_dir) = &toplevel_dir {
            if !file.starts_with(toplevel_dir) {
                ccprintln(window, "Found different toplevel directory");
                return Ok(false);
            }
        } else {
            // First iteration
            let comp: PathBuf = file.components().take(1).collect();
            ccprintln!(window, "Checking if path component {} is the only toplevel directory", comp.display());
            toplevel_dir = Some(comp);
        }
    }
    ccprintln(window, "Found no other toplevel directory");
    Ok(true)
}
