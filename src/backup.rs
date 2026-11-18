use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn backup_prefix(path: &Path) -> Option<String> {
    Some(format!("{}.bak.", path.file_name()?.to_string_lossy()))
}

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

// the stamp is bumped past the newest backup so the latest one always sorts last
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
