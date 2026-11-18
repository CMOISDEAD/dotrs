use std::{
    fs,
    path::{Path, PathBuf},
};

use dotrs::{
    Context, Error,
    backup::list_backups,
    commands::{self, Transfer},
    ignore::Ignore,
    scan::{self, FileStatus},
};
use tempfile::TempDir;

struct Env {
    _tmp: TempDir,
    ctx: Context,
}

impl Env {
    fn new() -> Env {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::with_home(tmp.path());
        fs::create_dir_all(&ctx.dots).unwrap();
        Env { _tmp: tmp, ctx }
    }

    fn home(&self, rel: &str) -> PathBuf {
        self.ctx.home.join(rel)
    }

    fn dots(&self, rel: &str) -> PathBuf {
        self.ctx.dots.join(rel)
    }

    fn write_home(&self, rel: &str, content: &str) {
        write(&self.home(rel), content);
    }

    fn write_dots(&self, rel: &str, content: &str) {
        write(&self.dots(rel), content);
    }

    fn add(&self, paths: &[&str]) -> dotrs::Result<()> {
        commands::add(&self.ctx, &to_paths(paths))
    }

    fn apply(&self, paths: &[&str], force: bool, dry_run: bool) -> dotrs::Result<()> {
        let paths = to_paths(paths);
        commands::apply(
            &self.ctx,
            Transfer {
                paths: &paths,
                dry_run,
                force,
            },
        )
    }

    fn sync(&self, paths: &[&str], force: bool, dry_run: bool) -> dotrs::Result<()> {
        let paths = to_paths(paths);
        commands::sync(
            &self.ctx,
            Transfer {
                paths: &paths,
                dry_run,
                force,
            },
        )
    }

    fn status_of(&self, rel: &str) -> FileStatus {
        let mut state = self.ctx.load_state();
        scan::scan(&self.ctx, &mut state)
            .into_iter()
            .find(|f| f.relative_path == Path::new(rel))
            .unwrap_or_else(|| panic!("{rel} is not managed"))
            .status
    }

    fn managed(&self) -> Vec<PathBuf> {
        scan::managed_files(&self.ctx.dots)
    }
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

fn to_paths(paths: &[&str]) -> Vec<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

#[test]
fn add_copies_file_into_dots_and_marks_it_clean() {
    let env = Env::new();
    env.write_home(".config/app/conf", "v1");

    env.add(&[".config/app/conf"]).unwrap();

    assert_eq!(read(&env.dots(".config/app/conf")), "v1");
    assert_eq!(env.status_of(".config/app/conf"), FileStatus::Clean);
}

#[test]
fn add_accepts_absolute_paths_in_home() {
    let env = Env::new();
    env.write_home(".bashrc", "x");

    let abs = env.home(".bashrc");
    env.add(&[abs.to_str().unwrap()]).unwrap();

    assert_eq!(env.managed(), [PathBuf::from(".bashrc")]);
}

#[test]
fn add_directory_recursively_skipping_symlinks_and_ignored() {
    let env = Env::new();
    env.write_home(".config/app/a", "a");
    env.write_home(".config/app/sub/b", "b");
    env.write_home(".config/app/.git/HEAD", "ref");
    std::os::unix::fs::symlink(env.home(".config/app/a"), env.home(".config/app/link")).unwrap();

    env.add(&[".config/app"]).unwrap();

    assert_eq!(
        env.managed(),
        [
            PathBuf::from(".config/app/a"),
            PathBuf::from(".config/app/sub/b")
        ]
    );
}

#[test]
fn add_rejects_symlinks_outside_paths_and_dots_dir() {
    let env = Env::new();
    env.write_home("real", "x");
    std::os::unix::fs::symlink(env.home("real"), env.home("link")).unwrap();
    env.write_dots("inside", "x");

    assert!(env.add(&["link"]).is_err());
    assert!(env.add(&["/etc/hostname"]).is_err());
    assert!(env.add(&["dots/inside"]).is_err());
    assert!(env.add(&["does-not-exist"]).is_err());
    assert!(env.add(&[env.ctx.home.to_str().unwrap()]).is_err());
    assert!(env.managed() == [PathBuf::from("inside")]);
}

#[test]
fn add_reports_each_failure_and_keeps_going() {
    let env = Env::new();
    env.write_home("good", "x");

    let err = env.add(&["missing1", "good", "missing2"]).unwrap_err();

    assert!(matches!(err, Error::Failed(2)));
    assert_eq!(env.managed(), [PathBuf::from("good")]);
}

#[test]
fn remove_never_escapes_the_dots_directory() {
    let env = Env::new();
    env.write_home(".bashrc", "precious");
    env.write_home(".config/app/conf", "precious");
    env.add(&[".config/app/conf"]).unwrap();

    let abs = env.home(".bashrc");
    for arg in [
        "../.bashrc",
        abs.to_str().unwrap(),
        "/etc/passwd",
        "dots/../.bashrc",
    ] {
        assert!(
            commands::remove(&env.ctx, &to_paths(&[arg])).is_err(),
            "{arg}"
        );
    }
    assert!(commands::remove(&env.ctx, std::slice::from_ref(&env.ctx.home)).is_err());

    assert_eq!(read(&env.home(".bashrc")), "precious");
    assert_eq!(read(&env.home(".config/app/conf")), "precious");
    assert_eq!(env.managed().len(), 1);
}

#[test]
fn remove_deletes_from_dots_only_and_cleans_empty_dirs() {
    let env = Env::new();
    env.write_home(".config/app/sub/conf", "x");
    env.write_home(".config/other", "y");
    env.add(&[".config/app/sub/conf", ".config/other"]).unwrap();

    commands::remove(&env.ctx, &to_paths(&[".config/app"])).unwrap();

    assert_eq!(env.managed(), [PathBuf::from(".config/other")]);
    assert!(!env.dots(".config/app").exists());
    assert!(env.dots(".config").exists());
    assert!(env.ctx.dots.exists());
    assert_eq!(read(&env.home(".config/app/sub/conf")), "x");
}

#[test]
fn remove_refuses_ignored_files() {
    let env = Env::new();
    env.write_dots(".git/HEAD", "ref");

    assert!(commands::remove(&env.ctx, &to_paths(&["dots/.git/HEAD"])).is_err());
    assert!(env.dots(".git/HEAD").exists());
}

#[test]
fn builtin_and_custom_ignore_patterns() {
    let env = Env::new();
    env.write_dots(
        ".dotrsignore",
        "# comment\n*.swp\n/scripts\n.config/secret/\n",
    );
    let ignore = Ignore::load(&env.ctx.dots);

    let ignored = |p: &str| ignore.is_ignored(Path::new(p));

    assert!(ignored(".git"));
    assert!(ignored(".config/nvim/.git/config"));
    assert!(ignored("README.md"));
    assert!(ignored("LICENSE"));
    assert!(ignored(".dotrsignore"));
    assert!(
        !ignored(".config/app/README.md"),
        "README is only ignored at the root"
    );
    assert!(ignored(".vimrc.swp"));
    assert!(ignored(".config/a/b.swp"));
    assert!(ignored("scripts/install.sh"));
    assert!(!ignored(".config/scripts/x"), "/scripts is anchored");
    assert!(ignored(".config/secret/token"));
    assert!(!ignored(".config/app/conf"));
}

#[test]
fn ignored_repository_files_are_never_applied() {
    let env = Env::new();
    env.write_dots("README.md", "docs");
    env.write_dots(".gitignore", "x");
    env.write_dots(".bashrc", "bash");

    env.apply(&[], false, false).unwrap();

    assert!(!env.home("README.md").exists());
    assert!(!env.home(".gitignore").exists());
    assert_eq!(read(&env.home(".bashrc")), "bash");
}

#[test]
fn status_detects_the_direction_of_every_change() {
    let env = Env::new();
    for f in ["missing", "modified", "updated", "conflict", "clean"] {
        env.write_home(f, "base");
    }
    env.add(&["missing", "modified", "updated", "conflict", "clean"])
        .unwrap();

    fs::remove_file(env.home("missing")).unwrap();
    env.write_home("modified", "home");
    env.write_dots("updated", "repo");
    env.write_home("conflict", "home");
    env.write_dots("conflict", "repo");
    env.write_dots("unknown", "never synced");
    env.write_home("unknown", "different");

    assert_eq!(env.status_of("missing"), FileStatus::Missing);
    assert_eq!(env.status_of("modified"), FileStatus::Modified);
    assert_eq!(env.status_of("updated"), FileStatus::Updated);
    assert_eq!(env.status_of("conflict"), FileStatus::Conflict);
    assert_eq!(env.status_of("clean"), FileStatus::Clean);
    assert_eq!(env.status_of("unknown"), FileStatus::Conflict);
}

#[test]
fn identical_files_are_recorded_so_later_changes_have_a_direction() {
    let env = Env::new();
    env.write_dots(".bashrc", "same");
    env.write_home(".bashrc", "same");

    commands::status(&env.ctx, false).unwrap();
    env.write_home(".bashrc", "edited");

    assert_eq!(env.status_of(".bashrc"), FileStatus::Modified);
}

#[test]
fn status_exit_code_reports_pending_changes() {
    let env = Env::new();
    env.write_home(".bashrc", "x");
    env.add(&[".bashrc"]).unwrap();

    assert!(commands::status(&env.ctx, true).is_ok());

    env.write_home(".bashrc", "y");
    assert!(matches!(
        commands::status(&env.ctx, true),
        Err(Error::Exit(1))
    ));
}

#[test]
fn home_entry_that_is_not_a_file_is_an_error() {
    let env = Env::new();
    env.write_dots("thing", "x");
    fs::create_dir_all(env.home("thing")).unwrap();

    assert_eq!(env.status_of("thing"), FileStatus::Error);
    assert!(env.apply(&[], true, false).is_err());
}

#[test]
fn apply_writes_missing_and_updated_and_skips_local_changes() {
    let env = Env::new();
    for f in ["missing", "modified", "updated", "conflict"] {
        env.write_home(f, "base");
    }
    env.add(&["missing", "modified", "updated", "conflict"])
        .unwrap();
    fs::remove_file(env.home("missing")).unwrap();
    env.write_home("modified", "home");
    env.write_dots("updated", "repo");
    env.write_home("conflict", "home");
    env.write_dots("conflict", "repo");

    env.apply(&[], false, false).unwrap();

    assert_eq!(read(&env.home("missing")), "base");
    assert_eq!(read(&env.home("updated")), "repo");
    assert_eq!(read(&env.home("modified")), "home", "local change kept");
    assert_eq!(read(&env.home("conflict")), "home", "conflict kept");
    assert_eq!(env.status_of("updated"), FileStatus::Clean);

    env.apply(&["conflict"], true, false).unwrap();
    assert_eq!(read(&env.home("conflict")), "repo");
    assert_eq!(
        read(&env.home("modified")),
        "home",
        "only the selected file"
    );
}

#[test]
fn apply_backs_up_overwritten_files() {
    let env = Env::new();
    env.write_home(".bashrc", "v1");
    env.add(&[".bashrc"]).unwrap();
    env.write_dots(".bashrc", "v2");

    env.apply(&[], false, false).unwrap();

    let backups = list_backups(&env.home(".bashrc"));
    assert_eq!(backups.len(), 1);
    assert_eq!(read(&backups[0].1), "v1");
}

#[test]
fn apply_dry_run_changes_nothing() {
    let env = Env::new();
    env.write_dots(".bashrc", "repo");

    env.apply(&[], false, true).unwrap();

    assert!(!env.home(".bashrc").exists());
}

#[test]
fn apply_unknown_filter_is_an_error() {
    let env = Env::new();
    env.write_dots(".bashrc", "x");

    assert!(env.apply(&["nope"], false, false).is_err());
    assert!(!env.home(".bashrc").exists());
}

#[test]
fn sync_copies_local_changes_and_skips_repo_changes() {
    let env = Env::new();
    for f in ["modified", "updated", "conflict"] {
        env.write_home(f, "base");
    }
    env.add(&["modified", "updated", "conflict"]).unwrap();
    env.write_home("modified", "home");
    env.write_dots("updated", "repo");
    env.write_home("conflict", "home");
    env.write_dots("conflict", "repo");

    env.sync(&[], false, true).unwrap();
    assert_eq!(read(&env.dots("modified")), "base", "dry run");

    env.sync(&[], false, false).unwrap();
    assert_eq!(read(&env.dots("modified")), "home");
    assert_eq!(read(&env.dots("updated")), "repo", "repo change kept");
    assert_eq!(read(&env.dots("conflict")), "repo", "conflict kept");
    assert_eq!(env.status_of("modified"), FileStatus::Clean);

    env.sync(&["conflict"], true, false).unwrap();
    assert_eq!(read(&env.dots("conflict")), "home");
}

#[test]
fn backups_never_overwrite_each_other() {
    let env = Env::new();
    env.write_home(".bashrc", "v1");
    env.add(&[".bashrc"]).unwrap();

    for (i, version) in ["v2", "v3", "v4"].iter().enumerate() {
        env.write_dots(".bashrc", version);
        env.apply(&[], false, false).unwrap();
        assert_eq!(list_backups(&env.home(".bashrc")).len(), i + 1);
    }

    let contents: Vec<String> = list_backups(&env.home(".bashrc"))
        .iter()
        .map(|(_, p)| read(p))
        .collect();
    assert_eq!(contents, ["v1", "v2", "v3"]);
}

#[test]
fn restore_brings_back_the_latest_backup_and_can_be_undone() {
    let env = Env::new();
    env.write_home(".bashrc", "mine");
    env.add(&[".bashrc"]).unwrap();
    env.write_dots(".bashrc", "repo");
    env.apply(&[], false, false).unwrap();
    assert_eq!(read(&env.home(".bashrc")), "repo");

    commands::restore(&env.ctx, &to_paths(&[".bashrc"])).unwrap();
    assert_eq!(read(&env.home(".bashrc")), "mine");

    commands::restore(&env.ctx, &to_paths(&[".bashrc"])).unwrap();
    assert_eq!(read(&env.home(".bashrc")), "repo");
}

#[test]
fn restore_without_backup_fails() {
    let env = Env::new();
    env.write_home(".bashrc", "x");

    assert!(commands::restore(&env.ctx, &to_paths(&[".bashrc"])).is_err());
    assert_eq!(read(&env.home(".bashrc")), "x");
}
