use crate::{ccprintln, ccprintlne, ccprintlnr};
use std::{
    error::Error,
    fmt, fs,
    io::{copy, Read, Seek},
    path::{Path, PathBuf, StripPrefixError},
};
use tauri::Window;
use zip::{result::ZipError, ZipArchive};

// Code taken from https://github.com/MCOfficer/zip-extract
// License: MIT
// Code taken due to lack up updates, a few prominent bugs & a lack of eyes from the community (potential security flaw)
// As a result, the code has been patched and debugging as been better integrated into the GUI

#[derive(Clone, Debug)]
pub struct StripToplevel {
    pub toplevel: PathBuf,
    pub path: PathBuf,
    pub error: StripPrefixError,
}

impl fmt::Display for StripToplevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to strip the top level ({}) from {}", self.toplevel.display(), self.path.display())
    }
}

impl Error for StripToplevel {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

pub fn extract<S: Read + Seek>(window: &Window, source: S, target_dir: &Path, strip_toplevel: bool, replace: bool) -> Result<(), Box<dyn Error>> {
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
    }

    let mut archive = ZipArchive::new(source)?;

    let do_strip_toplevel = strip_toplevel && has_toplevel(window, &mut archive)?;

    ccprintln(window, format!("Extracting to {}", target_dir.to_string_lossy()));
    ccprintln(window, "".to_owned());
    for i in 0..archive.len() {
        let mut item = archive.by_index(i)?;
        let mut relative_path = match item.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        if do_strip_toplevel {
            let base = relative_path.components().take(1).fold(PathBuf::new(), |mut p, c| {
                p.push(c);
                p
            });
            relative_path = relative_path.strip_prefix(&base).map_err(|error| StripToplevel {
                toplevel: base,
                path: relative_path.to_path_buf(),
                error,
            })?;
        }

        if relative_path.to_string_lossy().is_empty() {
            // Top-level directory
            continue;
        }

        let outpath = target_dir.join(relative_path);
        if item.is_dir() {
            ccprintlnr(window, format!("Creating directory {} from {}", outpath.to_string_lossy(), relative_path.display()));
            if !outpath.exists() {
                fs::create_dir_all(&outpath)?;
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
                    ccprintlne(window, format!("Failed to create directory {}: {}", p.display(), e));
                }
            }
        }

        ccprintlnr(window, format!("Creating {} from {}", outpath.to_string_lossy(), relative_path.display()));
        let mut outfile = fs::File::create(&outpath)?;
        copy(&mut item, &mut outfile)?;
    }

    ccprintlnr(window, format!("Extracted {} items", archive.len()));
    Ok(())
}

fn has_toplevel<S: Read + Seek>(window: &Window, archive: &mut ZipArchive<S>) -> Result<bool, ZipError> {
    let mut toplevel_dir: Option<PathBuf> = None;
    if archive.len() < 2 {
        return Ok(false);
    }

    for i in 0..archive.len() {
        let file = archive.by_index(i)?.mangled_name();
        if let Some(toplevel_dir) = &toplevel_dir {
            if !file.starts_with(toplevel_dir) {
                ccprintln(window, "Found different toplevel directory".to_owned());
                return Ok(false);
            }
        } else {
            // First iteration
            let comp: PathBuf = file.components().take(1).collect();
            ccprintln(window, format!("Checking if path component {} is the only toplevel directory", comp.display()));
            toplevel_dir = Some(comp);
        }
    }
    ccprintln(window, "Found no other toplevel directory".to_owned());
    Ok(true)
}
