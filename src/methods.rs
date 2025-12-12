use std::{
    env::home_dir,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

pub fn init() {
    let dots_dir = Path::new("/home/doom/dots");
    let result = fs::create_dir(dots_dir);
    match result {
        Ok(_) => println!("dotfiles initialized"),
        Err(e) => match e.kind() {
            ErrorKind::AlreadyExists => {
                println!("dotfiles already initialized")
            }
            ErrorKind::PermissionDenied => {
                println!("Permission denied")
            }
            _ => {
                println!("Error dir creation: {}", e)
            }
        },
    }

    let output = Command::new("git")
        .arg("init")
        .current_dir(dots_dir)
        .output();
    match output {
        Ok(_) => println!("Git initialized"),
        Err(e) => println!("Error git: {}", e),
    }
}

pub fn add(filename: Option<String>) {
    let dots_dir = Path::new("/home/doom/dots");

    let filename = filename.expect("empty file parameter");

    let home_dir = dirs_next::home_dir().expect("Could not get HOME directory");

    let absolute_path = match fs::canonicalize(&filename) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot resolve file '{}': {}", filename, e);
            return;
        }
    };

    if !absolute_path.starts_with(&home_dir) {
        eprintln!(
            "Error: the file must be inside HOME.\nFile: {}\nHOME: {}",
            absolute_path.display(),
            home_dir.display()
        );
        return;
    }

    let relative_path = match absolute_path.strip_prefix(&home_dir) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("Could not strip HOME prefix. Something is wrong.");
            return;
        }
    };

    let destination_path: PathBuf = dots_dir.join(relative_path);

    if let Some(parent) = destination_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!(
                "Error creating directory structure {}: {}",
                parent.display(),
                e
            );
            return;
        }
    }

    match fs::copy(&absolute_path, &destination_path) {
        Ok(_) => {
            println!(
                "Copied:\n  FROM {}\n  TO   {}",
                absolute_path.display(),
                destination_path.display()
            );
        }
        Err(e) => {
            eprintln!("Error copying file: {}", e);
        }
    }
}

pub fn apply() {
    let dots_dir = Path::new("/home/doom/dots");

    let home_dir = match home_dir() {
        Some(path) => path,
        None => {
            println!("There is no home?");
            return;
        }
    };

    let files = recursive_list(dots_dir);

    for file in files {
        if let Ok(relative) = file.strip_prefix(dots_dir) {
            let destination = home_dir.join(relative);

            if file.is_dir() {
                if let Err(e) = fs::create_dir_all(&destination) {
                    println!("Error creating directory {:?}: {}", destination, e);
                }
                continue;
            }

            if let Some(parent) = destination.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    println!("Error creating parent dirs {:?}: {}", parent, e);
                    continue;
                }
            }

            match fs::copy(&file, &destination) {
                Ok(_) => println!("Copied: {:?} -> {:?}", file, destination),
                Err(e) => println!("Error copying {:?}: {}", file, e),
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
