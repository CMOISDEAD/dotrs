use std::{
    env::home_dir,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

use color_print::cprintln;

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

    let files = recursive_list(dots_dir);

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

fn recursive_list(path: &Path) -> Vec<PathBuf> {
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
