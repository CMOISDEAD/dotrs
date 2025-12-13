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

## Usage

```sh
Usage: dotrs [OPTIONS] --action <ACTION>

Options:
  -a, --action <ACTION>  [possible values: init, apply, add, remove, list, status]
  -f, --file <FILE>
  -h, --help             Print help
  -V, --version          Print version
```

- Initialize dotfiles

```shell
dotrs --action init
```

Creates a ~/dots directory and initializes a Git repository inside it.

- Check status

```shell
dotrs --action status
```

```shell
Legend: + missing | M modified | = clean | ! error
= .config/noctalia/templates/pywalfox-colors.json
= .config/noctalia/templates/zathura-colors
= .config/niri/config.kdl
= .config/alacritty/alacritty.toml
= .config/ghostty/config
= .config/rmpc/config.ron
```

- Add a file.

```shell
dotrs --action add --file alacritty.toml
```

```shell
~/dots/
.
└── .config
    └── alacritty
        └── alacritty.toml
```

- Apply dotfiles.

```shell
dotrs --action apply
```
