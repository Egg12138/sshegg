# ssher

CLI + TUI for managing SSH sessions.

## Usage

Add a session:

```sh
cargo run -- add --name office --host office.example.com --user me --port 2222
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

## Configuration

Default session store path: `~/.config/ssher/sessions.json`

Override with:

- `--store-path /custom/path/sessions.json`
- `SSHER_STORE=/custom/path/sessions.json`

When you start the TUI with an empty store, it will prompt you to create the first session.
