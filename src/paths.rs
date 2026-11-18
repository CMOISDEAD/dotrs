//! Path handling: normalization and mapping command line arguments to managed paths.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    context::Context,
    error::{Result, msg},
};

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
    ctx: &Context,
    arg: &Path,
    accept: impl Fn(Root, &Path) -> bool,
    what: &str,
) -> Result<(Root, PathBuf)> {
    let (home, dots) = (ctx.home.as_path(), ctx.dots.as_path());
    let candidates = if arg.is_absolute() {
        vec![arg.to_path_buf()]
    } else {
        vec![ctx.cwd.join(arg), home.join(arg)]
    };

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
