# dotrs

<p align="center">
  <img src="https://github.com/user-attachments/assets/cb07e5ab-b87e-4473-9824-12e02e1fcb25" width="240" alt="dotrs logo"/>
</p>

**dotrs** is a minimalist dotfiles manager focused on practicality and simplicity.

- No bloated features.
- Straightforward workflow.
- Clean and fast.

Built for users who want full control over their environment with minimal overhead.

## Design goals

- Keep dotfile management simple and predictable.
- Avoid symlinks and hidden magic: dotfiles are plain copies.
- Make every action explicit and reversible.
- Follow a clean, UNIX-like workflow.

## Usage

```text
Usage: dotrs <COMMAND>

Commands:
  init     Create the dots directory, its .dotrsignore and a git repository
  add      Copy files or directories from HOME into the dots directory
  remove   Stop managing files (deletes them from the dots directory, never from HOME)
  list     List managed files
  status   Show the state of every managed file
  diff     Show differences between the dots directory (-) and HOME (+)
  apply    Copy dotfiles into HOME (missing files and files updated in the repository)
  sync     Copy files changed in HOME back into the dots directory
  restore  Restore the latest backup of files in HOME (run again to undo)
```

Typical workflow: `init → add → status → diff → apply / sync`

Paths can be given as `~/.config/foo`, as `.config/foo` from any directory,
or relative to the current directory.

### Initialize

```sh
$ dotrs init
✓ created /home/user/dots
✓ created .dotrsignore
✓ git repository initialized
```

### Add files

```sh
$ dotrs add ~/.config/alacritty ~/.bashrc
+ added .config/alacritty/alacritty.toml
+ added .bashrc
```

Directories are added recursively. Symlinks are skipped (dotrs never manages
symlinks), and files matching `.dotrsignore` are left out.

### Status

dotrs remembers the hash of every file at the last sync, so it knows **which
side** changed:

```sh
$ dotrs status
Legend: + missing | M modified in HOME | U updated in repo | C conflict | = clean | ! error
= .bashrc
M .config/niri/config.kdl
U .config/alacritty/alacritty.toml
C .config/ghostty/config
+ .config/rmpc/config.ron
```

| Symbol | Meaning | Fix with |
|---|---|---|
| `+` | missing in HOME | `apply` |
| `M` | changed in HOME | `sync` |
| `U` | changed in the repository (e.g. after `git pull`) | `apply` |
| `C` | changed on both sides, or never synced on this machine | `diff`, then `apply --force` or `sync --force` |
| `=` | clean | |
| `!` | error (unreadable, not a regular file…) | |

`dotrs status --exit-code` exits with 1 when something is not clean, which is
handy for scripts and shell prompts. Output is plain text when piped.

### Diff

```sh
$ dotrs diff .config/niri
```

Uses `git diff --no-index` (or `diff -u` when git is missing). Lines prefixed
with `-` are in the repository, `+` are in HOME.

### Apply

```sh
$ dotrs apply
B /home/user/.config/alacritty/alacritty.toml.bak.1763380000
→ .config/alacritty/alacritty.toml
→ .config/rmpc/config.ron
~ .config/niri/config.kdl (skipped: modified in HOME, run sync; use --force to override)
```

- Only `+` and `U` files are written; local changes are never overwritten
  unless you pass `--force`.
- Every overwritten file is backed up first as `<file>.bak.<timestamp>`.
  Backups are never overwritten.
- `--dry-run` (`-n`) shows what would happen; paths limit it to some files.

### Sync

```sh
$ dotrs sync
← synced .config/niri/config.kdl
```

Copies `M` files from HOME into the repository. Commit them with git as usual.

### Restore

```sh
$ dotrs restore ~/.config/alacritty/alacritty.toml
← restored .config/alacritty/alacritty.toml
```

Restores the most recent backup. The current file is backed up first, so
running `restore` again undoes it.

### Remove

```sh
$ dotrs remove .config/alacritty
- removed .config/alacritty/alacritty.toml
```

Deletes the file from the dots directory only. HOME is never touched.

## Configuration

| | |
|---|---|
| `DOTRS_DIR` | dots directory (default `~/dots`; relative paths are taken from HOME) |
| `~/dots/.dotrsignore` | files that are never applied (see below) |
| `$XDG_STATE_HOME/dotrs/state` | last synced hashes (default `~/.local/state/dotrs/state`), per machine, `sha256sum` format |
| `NO_COLOR` | disable colors |

### .dotrsignore

One pattern per line, lines starting with `#` are comments:

```text
# any file named *.swp, at any depth
*.swp
# a leading '/' anchors to the root of the dots directory
/scripts
# patterns containing '/' are always anchored
.config/secret/
```

`*` and `?` are supported. `.git`, `/.github`, `/.gitignore`,
`/.dotrsignore`, `/README*` and `/LICENSE*` are always ignored, so the
repository can hold its own README and license.

## Exit status

`0` on success, `1` if anything failed (each error is printed to stderr with
its cause) or, with `status --exit-code`, if any file is not clean.
