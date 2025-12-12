# dotrs

<p align="center">
  <img src="https://github.com/user-attachments/assets/cb07e5ab-b87e-4473-9824-12e02e1fcb25" width="240" alt="dotrs github logo image"/>
</p>

> [!NOTE]
> this project still on active development, is not ready for use...

**dotrs** is a minimalist dotfiles manager focused on practicality and simplicity.

- No bloated features.
- Straightforward configuration.
- Clean and fast workflow.

Perfect for users who want full control over their environment with minimal overhead.

## Usage

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
