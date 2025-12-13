use std::{env::home_dir, fs, io::ErrorKind, path::Path, process::Command};

use color_print::cprintln;

use crate::utils;

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

pub fn apply() {
    let dots_dir = Path::new("/home/doom/dots");

    let home_dir = match home_dir() {
        Some(p) => p,
        None => {
            cprintln!("<red>✗</> HOME not found");
            return;
        }
    };

    let files = utils::recursive_list(dots_dir);

    for file in files {
        if let Ok(relative) = file.strip_prefix(dots_dir) {
            let destination = home_dir.join(relative);

            if let Some(parent) = destination.parent() {
                let _ = fs::create_dir_all(parent);
            }

            match fs::copy(&file, &destination) {
                Ok(_) => cprintln!("<green>→</> {}", relative.display()),
                Err(_) => cprintln!("<red>✗</> {}", relative.display()),
            }
        }
    }
}

pub fn list() {
    let dots_dir = Path::new("/home/doom/dots");
    let files = utils::recursive_list(dots_dir);

    for file in files {
        if let Ok(relative) = file.strip_prefix(dots_dir) {
            cprintln!("<c>• {}</>", relative.to_string_lossy());
        }
    }
}

pub fn status() {
    cprintln!(
        "<dim>Legend:</> <cyan>+</> missing | <yellow>M</> modified | <green>=</> clean | <red>!</> error"
    );

    let dots_dir = Path::new("/home/doom/dots");

    let home_dir = match home_dir() {
        Some(p) => p,
        None => {
            cprintln!("<red>!</> HOME not found");
            return;
        }
    };

    let files = utils::recursive_list(dots_dir);

    for file in files {
        let relative = match file.strip_prefix(dots_dir) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let home_path = home_dir.join(relative);

        if !home_path.exists() {
            cprintln!("<cyan>+</> {}", relative.display());
            continue;
        }

        let dots_hash = match utils::calculate_sha256(&file) {
            Ok(h) => h,
            Err(_) => {
                cprintln!("<red>!</> {}", relative.display());
                continue;
            }
        };

        let home_hash = match utils::calculate_sha256(&home_path) {
            Ok(h) => h,
            Err(_) => {
                cprintln!("<red>!</> {}", relative.display());
                continue;
            }
        };

        if dots_hash == home_hash {
            cprintln!("<green>=</> {}", relative.display());
        } else {
            cprintln!("<yellow>M</> {}", relative.display());
        }
    }
}
