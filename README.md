# ssher

CLI + TUI for managing SSH sessions.

## User Guide

### Overview

`ssher` keeps your SSH sessions organized with a command-line tool for automation and an optional TUI for an interactive vim-inspired workflow. Use the CLI for scripts and the TUI for browsing, monitoring, and SCP helpers. Running `cargo run --` with no arguments also launches the TUI.

## Installation

### From Source (Recommended)

Clone the repository and run the install script:

```sh
git clone https://github.com/user/ssher.git
cd ssher
./scripts/install.sh
```

The install script will:
- Build the binary from source
- Install to `~/.local/bin` (or custom path with `--prefix`)
- Set up configuration directory with sample configs
- Install shell completions for bash/zsh/fish

**Install options:**

```sh
./scripts/install.sh --help          # Show all options
./scripts/install.sh --prefix ~/bin  # Custom install location
./scripts/install.sh --no-completions  # Skip shell completions
```

### With Cargo

```sh
cargo install --path .
```

### Development

Once installed you can still use `cargo run -- <command>` while iterating.

## Uninstallation

To remove ssher from your system:

```sh
# Remove the binary
rm ~/.local/bin/ssher

# Remove configuration and data (optional, keeps your sessions)
rm -rf ~/.config/ssher/

# Remove shell completions
rm ~/.local/share/bash-completion/completions/ssher 2>/dev/null
rm ~/.zfunc/_ssher 2>/dev/null
rm ~/.config/fish/completions/ssher.fish 2>/dev/null
```

If you used a custom `--prefix` during installation, adjust the binary path accordingly.

### CLI Commands

Add a session:

```sh
cargo run -- add --name office --host office.example.com --user me --port 2222
```

Add tags (repeat or comma-separated):

```sh
cargo run -- add --name office --host office.example.com --user me --tag prod,critical
```

List sessions:

```sh
cargo run -- list
```

Remove a session:

```sh
cargo run -- remove --name office
```

Copy files with SCP:

```sh
cargo run -- scp --name office --local ./file.txt --remote /tmp/file.txt
```

Launch the TUI:

```sh
cargo run -- tui
```

### TUI Navigation

- `j/k` (or arrow keys) move between sessions; `gg`/`G` jump to top/bottom; `Enter` connects.
- `/` starts search mode; type to filter, `Enter`/`Esc` exits.
- `o`/`O` opens the add-session form; `Up`, `Down`, `Tab`, and `Shift-Tab` move fields, `Enter` advances or submits (on the Tags line), and `Esc` cancels.
- `dd` starts delete confirmation; type the exact session name and hit `Enter`.
- `s` launches the SCP helper for the selected session.
- `m` toggles the monitor panel (active PIDs + last-connected).
- The bottom operation bar now combines the status line with the cheat sheet; if the focus line feels cramped, bump `layout.status_height` and keep `layout.help_height` sized for the help and navigation hints.

### Configuration

Default session store path: `~/.config/ssher/sessions.json`. Override with:

- `--store-path /custom/path/sessions.json`
- `SSHER_STORE=/custom/path/sessions.json`

Launching the TUI with an empty store prompts you to create the first session interactively.

## UI Configuration

Customize the TUI layout and theme with a JSON config file. Override the default `~/.config/ssher/ui.json` with:

- `--ui-config /custom/path/ui.json`
- `SSHER_UI_CONFIG=/custom/path/ui.json`

Key options:

- `layout.show_logo`, `layout.show_search`, `layout.show_monitor`: toggle panels.
- `layout.show_status`: include the status line inside the operation bar (focus + session count + connection count, plus any custom status text).
- `layout.status_height`: number of lines reserved for the status line inside the operation bar.
- `layout.help_height`: number of lines reserved for the cheat sheet (mode help + navigation); if `layout.show_help` is false only the navigation line stays visible.
- `theme.*`: control logo, header, border, status/info, help, and text colors.

CLI output colors can be customized via `~/.config/ssher/cli.json`, `--cli-config`, or `SSHER_CLI_CONFIG`.

Sample configs live in `assets/ui.sample.json` and `assets/cli.sample.json`.
