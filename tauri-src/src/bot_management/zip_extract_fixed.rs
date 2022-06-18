use std::{
    error::Error,
    fmt, fs,
    io::{copy, Read, Seek},
    path::{Path, PathBuf, StripPrefixError},
};

use zip::{result::ZipError, ZipArchive};

use crate::{ccprintln, ccprintlnr};

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

pub fn extract<S: Read + Seek>(source: S, target_dir: &Path, strip_toplevel: bool) -> Result<(), Box<dyn Error>> {
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
    }

    let mut archive = ZipArchive::new(source)?;

    let do_strip_toplevel = strip_toplevel && has_toplevel(&mut archive)?;

    ccprintln(format!("Extracting to {}", target_dir.to_string_lossy()));
    ccprintln("".to_string());
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut relative_path = match file.enclosed_name() {
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
        let outpath_str = outpath.to_string_lossy();

        ccprintlnr(format!("Creating {} from {}", outpath_str, relative_path.display()));
        if outpath_str.ends_with('/') || outpath_str.ends_with('\\') {
            if !outpath.exists() {
                fs::create_dir_all(&outpath)?;
            }
            continue;
        }

        if outpath.exists() {
            fs::remove_file(&outpath)?;
        } else if let Some(p) = outpath.parent() {
            fs::create_dir_all(&p).ok();
        }

        let mut outfile = fs::File::create(&outpath)?;
        copy(&mut file, &mut outfile)?;
    }

    ccprintlnr(format!("Extracted {} files", archive.len()));
    Ok(())
}

fn has_toplevel<S: Read + Seek>(archive: &mut ZipArchive<S>) -> Result<bool, ZipError> {
    let mut toplevel_dir: Option<PathBuf> = None;
    if archive.len() < 2 {
        return Ok(false);
    }

    for i in 0..archive.len() {
        let file = archive.by_index(i)?.mangled_name();
        if let Some(toplevel_dir) = &toplevel_dir {
            if !file.starts_with(toplevel_dir) {
                ccprintln("Found different toplevel directory".to_string());
                return Ok(false);
            }
        } else {
            // First iteration
            let comp: PathBuf = file.components().take(1).collect();
            ccprintln(format!("Checking if path component {} is the only toplevel directory", comp.display()));
            toplevel_dir = Some(comp);
        }
    }
    ccprintln("Found no other toplevel directory".to_string());
    Ok(true)
}
