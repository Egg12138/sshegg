# ssher

CLI + TUI for managing SSH sessions.

## Usage

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

Launch the TUI (or run with no arguments):

```sh
cargo run -- tui
```

TUI keys: `j/k` move, `gg/G` top/bottom, `/` search, `o/O` add, `s` scp, `dd` delete, `m` monitor, `Enter` connect.
Add session form: `Up/Down` or `Tab/Shift-Tab` move fields, `Enter` advance/submit, `Esc` cancel.

Copy files with SCP:

```sh
cargo run -- scp --name office --local ./file.txt --remote /tmp/file.txt
```

## Configuration

Default session store path: `~/.config/ssher/sessions.json`

Override with:

- `--store-path /custom/path/sessions.json`
- `SSHER_STORE=/custom/path/sessions.json`

When you start the TUI with an empty store, it will prompt you to create the first session.

## UI Configuration

You can customize the TUI layout and theme with a JSON config file.

Override with:

- `--ui-config /custom/path/ui.json`
- `SSHER_UI_CONFIG=/custom/path/ui.json`

Defaults load from `~/.config/ssher/ui.json` when present.

CLI output colors can be customized with `~/.config/ssher/cli.json` (or `--cli-config` / `SSHER_CLI_CONFIG`).

Sample configs live in `assets/ui.sample.json` and `assets/cli.sample.json`.
