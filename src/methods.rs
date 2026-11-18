use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    error::{Error, IoContext, Result, msg},
    ignore::{self, Ignore},
    output::{self, BLUE, CYAN, DIM, GREEN, MAGENTA, RED, YELLOW, paint},
    state::State,
    utils::{self, DotFile, FileStatus, Root},
};

/// Options shared by `apply` and `sync`.
pub struct Transfer<'a> {
    pub paths: &'a [PathBuf],
    pub dry_run: bool,
    pub force: bool,
}

fn save_state(state: &mut State) -> Result<()> {
    state.save().context(|| "cannot save state".into())
}

fn failures(failed: usize) -> Result<()> {
    if failed == 0 {
        Ok(())
    } else {
        Err(Error::Failed(failed))
    }
}

/// Copy `from` to `to`, creating parent directories as needed.
fn copy_file(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).context(|| format!("cannot create {}", parent.display()))?;
    }
    fs::copy(from, to).context(|| format!("cannot copy {} to {}", from.display(), to.display()))?;
    Ok(())
}

/// Copy `path` to a new timestamped backup and return the backup path.
fn backup(path: &Path) -> Result<PathBuf> {
    let backup_path = utils::next_backup_path(path);
    fs::copy(path, &backup_path).context(|| format!("cannot back up {}", path.display()))?;
    Ok(backup_path)
}

/// Resolve command line paths to managed paths (files or directories) and keep
/// only the scanned files under them. No paths means every file.
fn select(
    files: Vec<DotFile>,
    paths: &[PathBuf],
    home: &Path,
    dots: &Path,
) -> Result<Vec<DotFile>> {
    if paths.is_empty() {
        return Ok(files);
    }

    let mut filters = Vec::new();
    for arg in paths {
        let (_, rel) = utils::resolve(
            arg,
            home,
            dots,
            |_, rel| files.iter().any(|f| f.relative_path.starts_with(rel)),
            "not managed",
        )?;
        filters.push(rel);
    }

    Ok(files
        .into_iter()
        .filter(|f| filters.iter().any(|p| f.relative_path.starts_with(p)))
        .collect())
}

fn status_symbol(status: FileStatus) -> String {
    match status {
        FileStatus::Missing => paint(CYAN, "+"),
        FileStatus::Modified => paint(YELLOW, "M"),
        FileStatus::Updated => paint(BLUE, "U"),
        FileStatus::Conflict => paint(MAGENTA, "C"),
        FileStatus::Clean => paint(GREEN, "="),
        FileStatus::Error => paint(RED, "!"),
    }
}

fn skip(file: &DotFile, hint: &str) {
    println!(
        "{} {} {}",
        paint(YELLOW, "~"),
        file.relative_path.display(),
        paint(DIM, format!("(skipped: {hint}; use --force to override)"))
    );
}

fn report_error(file: &DotFile) {
    output::error(file.error.as_deref().unwrap_or("unknown error"));
}

/// Create the dots directory, its ignore file and a git repository.
pub fn init() -> Result<()> {
    let (_, dots) = utils::dirs()?;

    if dots.is_dir() {
        println!("{} {} already exists", paint(YELLOW, "•"), dots.display());
    } else {
        fs::create_dir_all(&dots).context(|| format!("cannot create {}", dots.display()))?;
        println!("{} created {}", paint(GREEN, "✓"), dots.display());
    }

    let ignore_file = dots.join(ignore::IGNORE_FILE);
    if !ignore_file.exists() {
        fs::write(&ignore_file, ignore::TEMPLATE)
            .context(|| format!("cannot create {}", ignore_file.display()))?;
        println!("{} created {}", paint(GREEN, "✓"), ignore::IGNORE_FILE);
    }

    if dots.join(".git").exists() {
        println!("{} git repository already exists", paint(YELLOW, "•"));
        return Ok(());
    }

    let output = match Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&dots)
        .output()
    {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(msg("git not found: install git or run `git init` yourself"));
        }
        other => other.context(|| "cannot run git".into())?,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(msg(format!("git init failed: {}", stderr.trim())));
    }

    println!("{} git repository initialized", paint(GREEN, "✓"));
    Ok(())
}

/// Copy files (or whole directories) from HOME into the dots directory.
pub fn add(paths: &[PathBuf]) -> Result<()> {
    let (home, dots) = utils::dirs()?;
    let ignore = Ignore::load(&dots);
    let mut state = utils::load_state(&home);
    let mut failed = 0;

    for arg in paths {
        if let Err(e) = add_one(arg, &home, &dots, &ignore, &mut state) {
            output::error(e);
            failed += 1;
        }
    }

    save_state(&mut state)?;
    failures(failed)
}

fn add_one(arg: &Path, home: &Path, dots: &Path, ignore: &Ignore, state: &mut State) -> Result<()> {
    let (root, rel) = utils::resolve(
        arg,
        home,
        dots,
        |root, rel| root == Root::Dots || home.join(rel).symlink_metadata().is_ok(),
        "no such file",
    )?;

    if root == Root::Dots {
        return Err(msg(format!(
            "{}: is inside the dots directory",
            arg.display()
        )));
    }
    if rel.as_os_str().is_empty() {
        return Err(msg("refusing to add the whole HOME directory"));
    }
    if ignore.is_ignored(&rel) {
        return Err(msg(format!("{}: matches an ignore pattern", arg.display())));
    }

    let source = home.join(&rel);
    let meta = source
        .symlink_metadata()
        .context(|| format!("cannot read {}", source.display()))?;

    let mut files = Vec::new();
    let mut symlinks = Vec::new();
    if meta.is_symlink() {
        symlinks.push(rel);
    } else if meta.is_dir() {
        if dots.starts_with(&source) {
            return Err(msg(format!(
                "{}: contains the dots directory",
                arg.display()
            )));
        }
        utils::walk(home, &rel, ignore, &mut files, &mut symlinks);
        files.sort();
    } else if meta.is_file() {
        files.push(rel);
    } else {
        return Err(msg(format!("{}: not a regular file", arg.display())));
    }

    for link in &symlinks {
        output::warn(format!(
            "{}: skipped, symlinks are not managed",
            link.display()
        ));
    }
    if files.is_empty() {
        return Err(msg(format!("{}: nothing to add", arg.display())));
    }

    for file in files {
        let destination = dots.join(&file);
        copy_file(&home.join(&file), &destination)?;
        let hash = utils::calculate_sha256(&destination)
            .context(|| format!("cannot read {}", destination.display()))?;
        state.set(&file, &hash);
        println!("{} added {}", paint(GREEN, "+"), file.display());
    }
    Ok(())
}

/// Stop managing files: delete them from the dots directory. HOME is never touched.
pub fn remove(paths: &[PathBuf]) -> Result<()> {
    let (home, dots) = utils::dirs()?;
    let managed = utils::managed_files(&dots);
    let mut state = utils::load_state(&home);
    let mut failed = 0;

    for arg in paths {
        let resolved = utils::resolve(
            arg,
            &home,
            &dots,
            |_, rel| !rel.as_os_str().is_empty() && managed.iter().any(|m| m.starts_with(rel)),
            "not managed",
        );
        let rel = match resolved {
            Ok((_, rel)) => rel,
            Err(e) => {
                output::error(e);
                failed += 1;
                continue;
            }
        };

        // `managed` only contains regular files found without following symlinks,
        // so every target is guaranteed to live inside the dots directory.
        for file in managed.iter().filter(|m| m.starts_with(&rel)) {
            let target = dots.join(file);
            if let Err(e) =
                fs::remove_file(&target).context(|| format!("cannot remove {}", target.display()))
            {
                output::error(e);
                failed += 1;
                continue;
            }
            remove_empty_parents(&target, &dots);
            state.remove(file);
            println!("{} removed {}", paint(RED, "-"), file.display());
        }
    }

    save_state(&mut state)?;
    failures(failed)
}

/// Remove empty directories above `path`, stopping at (and never removing) `root`.
fn remove_empty_parents(path: &Path, root: &Path) {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == root || !dir.starts_with(root) || fs::remove_dir(dir).is_err() {
            break;
        }
        current = dir.parent();
    }
}

pub fn list() -> Result<()> {
    let (_, dots) = utils::dirs()?;
    for file in utils::managed_files(&dots) {
        println!("{}", file.display());
    }
    Ok(())
}

/// Show the status of every managed file.
pub fn status(exit_code: bool) -> Result<()> {
    let (home, dots) = utils::dirs()?;
    let mut state = utils::load_state(&home);
    let files = utils::scan(&home, &dots, &mut state);

    if output::stdout_is_tty() {
        println!(
            "{} {} missing | {} modified in HOME | {} updated in repo | {} conflict | {} clean | {} error",
            paint(DIM, "Legend:"),
            status_symbol(FileStatus::Missing),
            status_symbol(FileStatus::Modified),
            status_symbol(FileStatus::Updated),
            status_symbol(FileStatus::Conflict),
            status_symbol(FileStatus::Clean),
            status_symbol(FileStatus::Error),
        );
    }

    for file in &files {
        println!(
            "{} {}",
            status_symbol(file.status),
            file.relative_path.display()
        );
    }
    for file in files.iter().filter(|f| f.status == FileStatus::Error) {
        report_error(file);
    }

    // record files found clean, so later changes have a known direction
    save_state(&mut state)?;

    if exit_code && files.iter().any(|f| f.status != FileStatus::Clean) {
        return Err(Error::Exit(1));
    }
    Ok(())
}

/// Show differences between the dots directory (-) and HOME (+).
pub fn diff(paths: &[PathBuf]) -> Result<()> {
    let (home, dots) = utils::dirs()?;
    let mut state = utils::load_state(&home);
    let files = select(utils::scan(&home, &dots, &mut state), paths, &home, &dots)?;
    let mut failed = 0;

    for file in files {
        let old = dots.join(&file.relative_path);
        let new = match file.status {
            FileStatus::Clean => continue,
            FileStatus::Error => {
                report_error(&file);
                failed += 1;
                continue;
            }
            FileStatus::Missing => PathBuf::from("/dev/null"),
            _ => home.join(&file.relative_path),
        };
        if let Err(e) = run_diff(&home, &old, &new) {
            output::error(e);
            failed += 1;
        }
    }

    failures(failed)
}

/// Show a unified diff with git (better output), falling back to diff(1).
/// Runs from HOME so the file labels are short (`dots/.bashrc` vs `.bashrc`).
fn run_diff(home: &Path, old: &Path, new: &Path) -> Result<()> {
    let old = old.strip_prefix(home).unwrap_or(old);
    let new = new.strip_prefix(home).unwrap_or(new);
    let _ = io::stdout().flush();
    let color = if output::color_stdout() {
        "--color=always"
    } else {
        "--color=never"
    };

    let status = match Command::new("git")
        .args(["--no-pager", "diff", "--no-index", color, "--"])
        .args([old, new])
        .current_dir(home)
        .status()
    {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Command::new("diff")
            .arg("-u")
            .args([old, new])
            .current_dir(home)
            .status()
            .context(|| "cannot run git or diff".into())?,
        other => other.context(|| "cannot run git".into())?,
    };

    // both tools exit with 1 when the files differ
    match status.code() {
        Some(0 | 1) => Ok(()),
        _ => Err(msg(format!("diff failed for {}", old.display()))),
    }
}

/// Copy dotfiles into HOME: files missing in HOME or updated in the repository.
pub fn apply(opts: Transfer) -> Result<()> {
    let (home, dots) = utils::dirs()?;
    let mut state = utils::load_state(&home);
    let files = select(
        utils::scan(&home, &dots, &mut state),
        opts.paths,
        &home,
        &dots,
    )?;
    let (mut done, mut failed) = (0, 0);

    for file in files {
        match file.status {
            FileStatus::Missing | FileStatus::Updated => {}
            FileStatus::Modified if !opts.force => {
                skip(&file, "modified in HOME, run sync");
                continue;
            }
            FileStatus::Conflict if !opts.force => {
                skip(&file, "changed on both sides, see diff");
                continue;
            }
            FileStatus::Modified | FileStatus::Conflict => {}
            FileStatus::Clean => continue,
            FileStatus::Error => {
                report_error(&file);
                failed += 1;
                continue;
            }
        }

        let rel = &file.relative_path;
        if opts.dry_run {
            println!("{} would apply {}", paint(DIM, "→"), rel.display());
            done += 1;
            continue;
        }

        let target = home.join(rel);
        let result = (|| {
            if file.status != FileStatus::Missing {
                let b = backup(&target)?;
                println!("{} {}", paint(DIM, "B"), b.display());
            }
            copy_file(&dots.join(rel), &target)
        })();

        match result {
            Ok(()) => {
                if let Some(hash) = &file.dots_hash {
                    state.set(rel, hash);
                }
                println!("{} {}", paint(GREEN, "→"), rel.display());
                done += 1;
            }
            Err(e) => {
                output::error(e);
                failed += 1;
            }
        }
    }

    if !opts.dry_run {
        save_state(&mut state)?;
    }
    if done == 0 && failed == 0 {
        println!("{}", paint(DIM, "nothing to apply"));
    }
    failures(failed)
}

/// Copy files changed in HOME back into the dots directory.
pub fn sync(opts: Transfer) -> Result<()> {
    let (home, dots) = utils::dirs()?;
    let mut state = utils::load_state(&home);
    let files = select(
        utils::scan(&home, &dots, &mut state),
        opts.paths,
        &home,
        &dots,
    )?;
    let (mut done, mut failed) = (0, 0);

    for file in files {
        match file.status {
            FileStatus::Modified => {}
            FileStatus::Updated if !opts.force => {
                skip(&file, "updated in the repository, run apply");
                continue;
            }
            FileStatus::Conflict if !opts.force => {
                skip(&file, "changed on both sides, see diff");
                continue;
            }
            FileStatus::Updated | FileStatus::Conflict => {}
            FileStatus::Missing | FileStatus::Clean => continue,
            FileStatus::Error => {
                report_error(&file);
                failed += 1;
                continue;
            }
        }

        let rel = &file.relative_path;
        if opts.dry_run {
            println!("{} would sync {}", paint(DIM, "←"), rel.display());
            done += 1;
            continue;
        }

        match copy_file(&home.join(rel), &dots.join(rel)) {
            Ok(()) => {
                if let Some(hash) = &file.home_hash {
                    state.set(rel, hash);
                }
                println!("{} synced {}", paint(GREEN, "←"), rel.display());
                done += 1;
            }
            Err(e) => {
                output::error(e);
                failed += 1;
            }
        }
    }

    if !opts.dry_run {
        save_state(&mut state)?;
    }
    if done == 0 && failed == 0 {
        println!("{}", paint(DIM, "nothing to sync"));
    }
    failures(failed)
}

/// Restore the most recent backup of files in HOME.
/// The current file is backed up first, so running restore again undoes it.
pub fn restore(paths: &[PathBuf]) -> Result<()> {
    let (home, dots) = utils::dirs()?;
    let mut failed = 0;

    for arg in paths {
        if let Err(e) = restore_one(arg, &home, &dots) {
            output::error(e);
            failed += 1;
        }
    }
    failures(failed)
}

fn restore_one(arg: &Path, home: &Path, dots: &Path) -> Result<()> {
    let (_, rel) = utils::resolve(
        arg,
        home,
        dots,
        |_, rel| !utils::list_backups(&home.join(rel)).is_empty(),
        "no backup found",
    )?;
    let target = home.join(&rel);
    let Some((_, latest)) = utils::list_backups(&target).pop() else {
        return Err(msg(format!("{}: no backup found", arg.display())));
    };

    if target.exists() {
        let b = backup(&target)?;
        println!("{} {}", paint(DIM, "B"), b.display());
    }

    fs::copy(&latest, &target).context(|| format!("cannot restore {}", target.display()))?;
    fs::remove_file(&latest).context(|| format!("cannot remove {}", latest.display()))?;

    println!("{} restored {}", paint(GREEN, "←"), rel.display());
    Ok(())
}
