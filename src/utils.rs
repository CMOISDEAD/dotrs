use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{self, BufReader, Read},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    error::{Result, msg},
    ignore::Ignore,
    state::State,
};

pub fn home_dir() -> Result<PathBuf> {
    env::home_dir()
        .filter(|p| p.is_absolute())
        .ok_or_else(|| msg("cannot determine HOME"))
}

/// `$DOTRS_DIR` (relative paths are taken from HOME), or `~/dots`.
pub fn dots_dir(home: &Path) -> PathBuf {
    match env::var_os("DOTRS_DIR").filter(|v| !v.is_empty()) {
        Some(dir) => normalize(&home.join(dir)),
        None => home.join("dots"),
    }
}

pub fn dirs() -> Result<(PathBuf, PathBuf)> {
    let home = home_dir()?;
    let dots = dots_dir(&home);
    Ok((home, dots))
}

pub fn load_state(home: &Path) -> State {
    State::load(State::default_path(home))
}

/// Files managed in `dots_dir`, as paths relative to it.
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

/// Collect regular files under `root/relative`, as paths relative to `root`.
/// Ignored entries are skipped; symlinks are never followed and are collected
/// separately in `symlinks`.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Root {
    Home,
    Dots,
}

/// Which root (dots directory or HOME) an absolute path lives in, and the path
/// relative to it. Symlinked HOME/dots directories are handled by also comparing
/// against their canonical form. Symlinks in the path itself are not resolved.
fn relative_to_roots(path: &Path, home: &Path, dots: &Path) -> Option<(Root, PathBuf)> {
    let path = normalize(path);
    let roots = [
        (Root::Dots, dots.to_path_buf()),
        (Root::Dots, fs::canonicalize(dots).unwrap_or_default()),
        (Root::Home, home.to_path_buf()),
        (Root::Home, fs::canonicalize(home).unwrap_or_default()),
    ];
    roots
        .into_iter()
        .filter(|(_, r)| !r.as_os_str().is_empty())
        .find_map(|(root, r)| Some((root, path.strip_prefix(r).ok()?.to_path_buf())))
}

/// Resolve a path given on the command line to a path relative to HOME
/// (equivalently, to the dots directory).
///
/// Relative paths are tried against the current directory first and then
/// against HOME; the first candidate accepted by `accept` wins.
pub fn resolve(
    arg: &Path,
    home: &Path,
    dots: &Path,
    accept: impl Fn(Root, &Path) -> bool,
    what: &str,
) -> Result<(Root, PathBuf)> {
    let mut candidates = Vec::new();
    if arg.is_absolute() {
        candidates.push(arg.to_path_buf());
    } else {
        if let Ok(cwd) = env::current_dir() {
            candidates.push(cwd.join(arg));
        }
        candidates.push(home.join(arg));
    }

    let mut inside = false;
    for candidate in candidates {
        if let Some((root, rel)) = relative_to_roots(&candidate, home, dots) {
            inside = true;
            if accept(root, &rel) {
                return Ok((root, rel));
            }
        }
    }

    if inside {
        Err(msg(format!("{}: {what}", arg.display())))
    } else {
        Err(msg(format!("{}: outside HOME", arg.display())))
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
    /// not present in HOME
    Missing,
    /// changed in HOME since the last sync
    Modified,
    /// changed in the dots repository since the last sync
    Updated,
    /// changed on both sides, or no previous sync recorded
    Conflict,
    Clean,
    Error,
}

pub struct DotFile {
    pub relative_path: PathBuf,
    pub status: FileStatus,
    pub dots_hash: Option<String>,
    pub home_hash: Option<String>,
    /// set when `status` is `Error`
    pub error: Option<String>,
}

/// Compare every managed file against HOME and the last synchronized hash.
/// Files found identical on both sides are recorded in `state`.
pub fn scan(home: &Path, dots: &Path, state: &mut State) -> Vec<DotFile> {
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
