use dirs_next::home_dir;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Result},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::ignore::Ignore;

/// Files managed in `dots_dir`, as paths relative to it.
/// Ignored entries and symlinks are skipped (symlinks are never followed).
pub fn managed_files(dots_dir: &Path) -> Vec<PathBuf> {
    let ignore = Ignore::load(dots_dir);
    let mut files = Vec::new();
    walk(dots_dir, Path::new(""), &ignore, &mut files);
    files.sort();
    files
}

fn walk(root: &Path, relative: &Path, ignore: &Ignore, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root.join(relative)) else {
        return;
    };

    for entry in entries.flatten() {
        let rel = relative.join(entry.file_name());
        if ignore.is_ignored(&rel) {
            continue;
        }
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk(root, &rel, ignore, files),
            Ok(t) if t.is_file() => files.push(rel),
            _ => {}
        }
    }
}

/// Lexically normalize a path: drop `.` and resolve `..` without touching the filesystem.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

pub fn calculate_sha256(path: &Path) -> Result<String> {
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

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    MISSING,
    MODIFIED,
    CLEAN,
    ERROR,
}

pub struct DotFile {
    pub relative_path: PathBuf,
    pub status: FileStatus,
}

pub fn scan_dots() -> Vec<DotFile> {
    let dots_dir = get_dots_dir();

    let home_dir = match home_dir() {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut results = Vec::new();

    for relative in managed_files(&dots_dir) {
        let file = dots_dir.join(&relative);
        let home_path = home_dir.join(&relative);

        // Case 1: file does not exist in HOME
        if !home_path.exists() {
            results.push(DotFile {
                relative_path: relative,
                status: FileStatus::MISSING,
            });
            continue;
        }

        // Case 2: hash comparison
        let status = match (calculate_sha256(&file), calculate_sha256(&home_path)) {
            (Ok(a), Ok(b)) if a == b => FileStatus::CLEAN,
            (Ok(_), Ok(_)) => FileStatus::MODIFIED,
            _ => FileStatus::ERROR,
        };

        results.push(DotFile {
            relative_path: relative,
            status,
        });
    }

    results
}

/// Backups live next to the original as `<name>.bak.<unix-seconds>`.
fn backup_prefix(path: &Path) -> Option<String> {
    Some(format!("{}.bak.", path.file_name()?.to_string_lossy()))
}

/// Existing backups of `path`, sorted oldest first.
pub fn list_backups(path: &Path) -> Vec<(u64, PathBuf)> {
    let (Some(parent), Some(prefix)) = (path.parent(), backup_prefix(path)) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };

    let mut backups: Vec<(u64, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stamp = name.strip_prefix(&prefix)?.parse().ok()?;
            Some((stamp, e.path()))
        })
        .collect();
    backups.sort();
    backups
}

/// A fresh backup path for `path`. Never reuses an existing backup and is always
/// newer than any existing one, so the latest backup is the one with the highest stamp.
pub fn next_backup_path(path: &Path) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let latest = list_backups(path).last().map(|(s, _)| *s);
    let stamp = match latest {
        Some(l) if l >= now => l + 1,
        _ => now,
    };

    let mut backup = path.as_os_str().to_owned();
    backup.push(format!(".bak.{stamp}"));
    PathBuf::from(backup)
}

pub fn get_dots_dir() -> PathBuf {
    let home_dir = dirs_next::home_dir().expect("No Home");

    home_dir.join("dots")
}
