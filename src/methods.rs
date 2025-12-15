use std::{env::home_dir, fs, io::ErrorKind, path::Path, process::Command};

use color_print::cprintln;

use crate::utils::{self, FileStatus};

pub fn init() {
    let dots_dir = Path::new("/home/doom/dots");

    match fs::create_dir(dots_dir) {
        Ok(_) => cprintln!("<green>✓</> dots directory created"),
        Err(e) => match e.kind() {
            ErrorKind::AlreadyExists => {
                cprintln!("<yellow>•</> dots directory already exists")
            }
            ErrorKind::PermissionDenied => {
                cprintln!("<red>✗</> permission denied");
                return;
            }
            _ => {
                cprintln!("<red>✗</> cannot create dots directory");
                return;
            }
        },
    }

    match Command::new("git")
        .arg("init")
        .current_dir(dots_dir)
        .output()
    {
        Ok(_) => cprintln!("<green>✓</> git repository initialized"),
        Err(_) => cprintln!("<red>✗</> git not available"),
    }
}

pub fn add(filename: Option<String>) {
    let dots_dir = Path::new("/home/doom/dots");
    let filename = filename.expect("missing file");

    let home_dir = dirs_next::home_dir().expect("no HOME");

    let absolute_path = match fs::canonicalize(&filename) {
        Ok(p) => p,
        Err(_) => {
            cprintln!("<red>✗</> file not found");
            return;
        }
    };

    if !absolute_path.starts_with(&home_dir) {
        cprintln!("<red>✗</> file must be inside HOME");
        return;
    }

    let relative = absolute_path.strip_prefix(&home_dir).unwrap();
    let destination = dots_dir.join(relative);

    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            cprintln!("<red>✗</> cannot create directory structure");
            return;
        }
    }

    match fs::copy(&absolute_path, &destination) {
        Ok(_) => {
            cprintln!("<green>✓</> added <dim>{}</>", relative.display());
        }
        Err(_) => cprintln!("<red>✗</> copy failed"),
    }
}

pub fn list() {
    let dots_dir = Path::new("/home/doom/dots");
    let files = utils::recursive_list(dots_dir);

    for file in files {
        if let Ok(relative) = file.strip_prefix(dots_dir) {
            cprintln!("<cyan>• {}</>", relative.to_string_lossy());
        }
    }
}

pub fn apply() {
    let dots_dir = Path::new("/home/doom/dots");

    let home_dir = match home_dir() {
        Some(p) => p,
        None => {
            cprintln!("<red>!</> HOME not found");
            return;
        }
    };

    for file in utils::scan_dots() {
        match file.status {
            FileStatus::MISSING | FileStatus::MODIFIED => {
                let source_path = dots_dir.join(&file.relative_path);
                let target_path = home_dir.join(&file.relative_path);

                if target_path.exists() {
                    let backup_path = utils::with_bak(&target_path);

                    match fs::copy(&target_path, &backup_path) {
                        Ok(_) => {
                            cprintln!("<dim>B</> {}", backup_path.display());
                        }
                        Err(_) => {
                            cprintln!("<red>!</> {}", file.relative_path.display());
                            continue;
                        }
                    }
                }

                if let Some(parent) = target_path.parent() {
                    if fs::create_dir_all(parent).is_err() {
                        cprintln!("<red>!</> {}", file.relative_path.display());
                        continue;
                    }
                }

                match fs::copy(&source_path, &target_path) {
                    Ok(_) => cprintln!("<green>→</> {}", file.relative_path.display()),
                    Err(_) => cprintln!("<red>!</> {}", file.relative_path.display()),
                }
            }
            _ => {}
        }
    }

    cprintln!("<dim>done</>");
}

pub fn status() {
    cprintln!(
        "<dim>Legend:</> <cyan>+</> missing | <yellow>M</> modified | <green>=</> clean | <red>!</> error"
    );

    for file in utils::scan_dots() {
        match file.status {
            FileStatus::MISSING => cprintln!("<cyan>+</> {}", file.relative_path.display()),
            FileStatus::MODIFIED => cprintln!("<yellow>M</> {}", file.relative_path.display()),
            FileStatus::CLEAN => cprintln!("<green>=</> {}", file.relative_path.display()),
            FileStatus::ERROR => cprintln!("<red>!</> {}", file.relative_path.display()),
        }
    }
}
