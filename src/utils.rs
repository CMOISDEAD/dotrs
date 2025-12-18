use dirs_next::home_dir;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Result},
    path::{Path, PathBuf},
};

pub fn recursive_list(path: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if !path.is_dir() {
        return files;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let entry_path = entry.path();

                if let Some(name) = entry.file_name().to_str() {
                    if name == ".git" {
                        continue;
                    }
                }

                if entry_path.is_dir() {
                    files.extend(recursive_list(&entry_path));
                } else {
                    files.push(entry_path);
                }
            }
        }
    }

    files
}

pub fn calculate_sha256(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];

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

    for file in recursive_list(&dots_dir) {
        let relative = match file.strip_prefix(&dots_dir) {
            Ok(p) => p.to_path_buf(),
            Err(_) => continue,
        };

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
        let dots_hash = match calculate_sha256(&file) {
            Ok(h) => h,
            Err(_) => {
                results.push(DotFile {
                    relative_path: relative,
                    status: FileStatus::ERROR,
                });
                continue;
            }
        };

        let home_hash = match calculate_sha256(&home_path) {
            Ok(h) => h,
            Err(_) => {
                results.push(DotFile {
                    relative_path: relative,
                    status: FileStatus::ERROR,
                });
                continue;
            }
        };

        let status = if dots_hash == home_hash {
            FileStatus::CLEAN
        } else {
            FileStatus::MODIFIED
        };

        results.push(DotFile {
            relative_path: relative,
            status,
        });
    }

    results
}

pub fn with_bak(path: &Path) -> PathBuf {
    let mut backup = path.as_os_str().to_owned();
    backup.push(".bak");
    PathBuf::from(backup)
}


pub fn get_dots_dir () -> PathBuf {
    let home_dir = dirs_next::home_dir().expect("No Home");

    home_dir.join("dots")
}
