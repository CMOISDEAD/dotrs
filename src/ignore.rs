use std::{
    fs,
    path::{Component, Path},
};

/// Name of the ignore file, stored at the root of the dots directory.
pub const IGNORE_FILE: &str = ".dotrsignore";

/// Patterns that are always ignored, regardless of the ignore file.
const BUILTIN: &[&str] = &[
    ".git",
    "/.github",
    "/.gitignore",
    "/.dotrsignore",
    "/README*",
    "/LICENSE*",
];

/// Template written by `init`.
pub const TEMPLATE: &str = "\
# dotrs ignore file: one pattern per line.
# Files matching these patterns are never applied to HOME.
#
#   name      matches a file or directory with that name at any depth
#   /path     matches only relative to the root of the dots directory
#   a/b       patterns containing '/' are always anchored to the root
#   *  ?      wildcards (never match '/')
#
# Always ignored: .git, /.github, /.gitignore, /.dotrsignore, /README*, /LICENSE*
";

struct Pattern {
    glob: String,
    anchored: bool,
}

pub struct Ignore {
    patterns: Vec<Pattern>,
}

impl Ignore {
    /// Built-in patterns plus the ones in `<dots>/.dotrsignore`, if present.
    pub fn load(dots_dir: &Path) -> Ignore {
        let content = fs::read_to_string(dots_dir.join(IGNORE_FILE)).unwrap_or_default();
        Ignore::from_lines(BUILTIN.iter().copied().chain(content.lines()))
    }

    fn from_lines<'a>(lines: impl Iterator<Item = &'a str>) -> Ignore {
        let patterns = lines
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let anchored = l.trim_end_matches('/').contains('/');
                let glob = l.trim_matches('/').to_string();
                (!glob.is_empty()).then_some(Pattern { glob, anchored })
            })
            .collect();
        Ignore { patterns }
    }

    /// `relative` is a path relative to the dots directory.
    pub fn is_ignored(&self, relative: &Path) -> bool {
        let parts: Vec<String> = relative
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();

        self.patterns.iter().any(|p| {
            if p.anchored {
                // match the pattern against every leading prefix (so directories match)
                (1..=parts.len()).any(|n| glob(&p.glob, &parts[..n].join("/")))
            } else {
                parts.iter().any(|part| glob(&p.glob, part))
            }
        })
    }
}

/// Minimal glob: `*` matches any run of characters except '/', `?` matches one.
fn glob(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    glob_at(&p, &t)
}

fn glob_at(p: &[char], t: &[char]) -> bool {
    match p.first() {
        None => t.is_empty(),
        Some('*') => glob_at(&p[1..], t) || (!t.is_empty() && t[0] != '/' && glob_at(p, &t[1..])),
        Some('?') => !t.is_empty() && t[0] != '/' && glob_at(&p[1..], &t[1..]),
        Some(c) => t.first() == Some(c) && glob_at(&p[1..], &t[1..]),
    }
}
