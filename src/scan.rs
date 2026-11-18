use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Path, PathBuf},
};

use crate::{context::Context, ignore::Ignore, state::State};

pub fn managed_files(dots_dir: &Path) -> Vec<PathBuf> {
    let ignore = Ignore::load(dots_dir);
    let mut files = Vec::new();
    walk(
        dots_dir,
        Path::new(""),
        &ignore,
        &mut files,
        &mut Vec::new(),
    );
    files.sort();
    files
}

pub fn walk(
    root: &Path,
    relative: &Path,
    ignore: &Ignore,
    files: &mut Vec<PathBuf>,
    symlinks: &mut Vec<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(root.join(relative)) else {
        return;
    };

    for entry in entries.flatten() {
        let rel = relative.join(entry.file_name());
        if ignore.is_ignored(&rel) {
            continue;
        }
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk(root, &rel, ignore, files, symlinks),
            Ok(t) if t.is_file() => files.push(rel),
            Ok(t) if t.is_symlink() => symlinks.push(rel),
            _ => {}
        }
    }
}

pub fn calculate_sha256(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Missing,
    Modified,
    Updated,
    Conflict,
    Clean,
    Error,
}

pub struct DotFile {
    pub relative_path: PathBuf,
    pub status: FileStatus,
    pub dots_hash: Option<String>,
    pub home_hash: Option<String>,
    pub error: Option<String>,
}

pub fn scan(ctx: &Context, state: &mut State) -> Vec<DotFile> {
    let (home, dots) = (ctx.home.as_path(), ctx.dots.as_path());
    managed_files(dots)
        .into_iter()
        .map(|relative| scan_file(home, dots, state, relative))
        .collect()
}

fn scan_file(home: &Path, dots: &Path, state: &mut State, relative: PathBuf) -> DotFile {
    let failed = |relative: PathBuf, error: String| DotFile {
        relative_path: relative,
        status: FileStatus::Error,
        dots_hash: None,
        home_hash: None,
        error: Some(error),
    };

    let dots_path = dots.join(&relative);
    let dots_hash = match calculate_sha256(&dots_path) {
        Ok(h) => h,
        Err(e) => {
            return failed(
                relative,
                format!("cannot read {}: {e}", dots_path.display()),
            );
        }
    };

    let home_path = home.join(&relative);
    let home_hash = match home_path.symlink_metadata() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Ok(m) if !m.is_file() => {
            let error = format!("{} is not a regular file", home_path.display());
            return failed(relative, error);
        }
        _ => match calculate_sha256(&home_path) {
            Ok(h) => Some(h),
            Err(e) => {
                return failed(
                    relative,
                    format!("cannot read {}: {e}", home_path.display()),
                );
            }
        },
    };

    let status = match &home_hash {
        None => FileStatus::Missing,
        Some(h) if *h == dots_hash => {
            state.set(&relative, h);
            FileStatus::Clean
        }
        Some(h) => match state.get(&relative) {
            Some(base) if base == h => FileStatus::Updated,
            Some(base) if base == dots_hash => FileStatus::Modified,
            _ => FileStatus::Conflict,
        },
    };

    DotFile {
        relative_path: relative,
        status,
        dots_hash: Some(dots_hash),
        home_hash,
        error: None,
    }
}
