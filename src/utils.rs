use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Result},
    path::{Path, PathBuf},
};
pub fn recursive_list(path: &Path) -> Vec<PathBuf> {
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
