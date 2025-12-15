# dotrs

<p align="center">
  <img src="https://github.com/user-attachments/assets/cb07e5ab-b87e-4473-9824-12e02e1fcb25" width="240" alt="dotrs logo"/>
</p>

> [!NOTE]
> This project is still under active development and is not ready for production use.

**dotrs** is a minimalist dotfiles manager focused on practicality and simplicity.

- No bloated features.
- Straightforward workflow.
- Clean and fast.

Built for users who want full control over their environment with minimal overhead.

## Design goals

- Keep dotfile management simple and predictable.
- Avoid symlinks and hidden magic.
- Make every action explicit and reversible.
- Follow a clean, UNIX-like workflow.

## Usage

```text
Usage: dotrs [OPTIONS] --action <ACTION>

Options:
  -a, --action <ACTION>  [possible values: init, apply, add, remove, list, status]
  -f, --file <FILE>
  -h, --help             Print help
  -V, --version          Print version
```

- Typical workflow:
`init → add → status → apply`

- Initialize dotfiles

```sh
dotrs --action init
```

Creates a ~/dots directory and initializes a Git repository inside it.

- Check status

```sh
dotrs --action status
```

```sh
Legend: + missing | M modified | = clean | ! error
= .config/noctalia/templates/pywalfox-colors.json
= .config/noctalia/templates/zathura-colors
= .config/niri/config.kdl
= .config/alacritty/alacritty.toml
= .config/ghostty/config
= .config/rmpc/config.ron
```

- Add a file

```sh
dotrs --action add --file alacritty.toml
```

```sh
~/dots/
.
└── .config
    └── alacritty
        └── alacritty.toml
```

- Apply dotfiles

```sh
dotrs --action apply
```

```sh
B .config/alacritty/alacritty.toml.bak
→ .config/alacritty/alacritty.toml
done
```

Automatically backs up modified files.
