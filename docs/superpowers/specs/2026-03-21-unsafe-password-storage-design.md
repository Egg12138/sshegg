# Unsafe Mode for Password Storage

## Overview

Add an "unsafe" mode for ssher to support password storage in environments where system keyring is unavailable. Three modes are supported:

- `"normal"` (default): Use system keyring (existing behavior)
- `"bare"`: Store password as plaintext in session file
- `"simple"`: Store password with XOR encoding using a configurable key

Supports both global setting and per-session override.

## File Format

### New `sessions.json` Structure

```json
{
  "passwd_unsafe_mode": "normal",
  "passwd_unsafe_key": null,
  "sessions": [
    {
      "name": "office",
      "host": "office.example.com",
      "user": "bob",
      "port": 2222,
      "identity_file": "/home/bob/.ssh/id_rsa",
      "tags": ["work"],
      "last_connected_at": 1234567890,
      "has_stored_password": false,
      "passwd_unsafe_mode": null,
      "stored_password": null
    }
  ]
}
```

### Fields

| Field | Level | Type | Description |
|-------|-------|------|-------------|
| `passwd_unsafe_mode` | root | string | Global setting: `"normal"` \| `"bare"` \| `"simple"`. Default: `"normal"` |
| `passwd_unsafe_key` | root | string \| null | Fallback XOR key if `SSHER_UNSAFE_KEY` env var not set |
| `sessions` | root | array | Array of session objects |
| `passwd_unsafe_mode` | session | string \| null | Per-session override. `null` = inherit from global |
| `stored_password` | session | string \| null | Password stored in unsafe format |

### Backward Compatibility

On load, detect if root is an array (old format) and auto-migrate:
- Wrap in new format with `passwd_unsafe_mode: "normal"` and `passwd_unsafe_key: null`
- Preserve all existing session fields
- Save migrated format on next write

## Password Storage & Retrieval

### Storage Logic

1. Get effective mode: session-level `passwd_unsafe_mode` ?? global `passwd_unsafe_mode` ?? `"normal"`
2. Based on mode:
   - `"normal"`: Store via keyring (existing behavior)
   - `"bare"`: Store plaintext in `stored_password` field
   - `"simple"`: XOR encode with key, store result in `stored_password`
3. Set `has_stored_password = true`

### Retrieval Logic

1. Get effective mode: session-level `passwd_unsafe_mode` ?? global `passwd_unsafe_mode` ?? `"normal"`
2. Based on mode:
   - `"normal"`: Retrieve from keyring
   - `"bare"`: Read `stored_password` directly
   - `"simple"`: Read `stored_password`, XOR decode with key
3. If retrieval fails, fall back to manual password prompt

### Key Resolution for "simple" Mode

```
SSHER_UNSAFE_KEY env var → passwd_unsafe_key from config → error if neither set
```

### XOR Implementation

```rust
fn xor_encode(password: &str, key: &str) -> String {
    let password_bytes = password.as_bytes();
    let key_bytes = key.as_bytes();
    let result: Vec<u8> = password_bytes
        .iter()
        .zip(key_bytes.iter().cycle())
        .map(|(p, k)| p ^ k)
        .collect();
    // Encode as base64 for safe JSON storage
    base64::encode(&result)
}

fn xor_decode(encoded: &str, key: &str) -> Result<String> {
    let bytes = base64::decode(encoded)?;
    let key_bytes = key.as_bytes();
    let result: Vec<u8> = bytes
        .iter()
        .zip(key_bytes.iter().cycle())
        .map(|(p, k)| p ^ k)
        .collect();
    String::from_utf8(result).context("decoded password is not valid UTF-8")
}
```

Note: XOR result is base64-encoded for safe JSON storage (avoids control characters).

## TUI Changes

### Session Form Fields

Current order: Name, Host, User, Port, Identity File, Tags, Password

New order:
1. Name
2. Host
3. User
4. Port
5. Identity File
6. Tags
7. Password (optional input)
8. **Password Storage Mode** (dropdown - shown only if password entered)
   - Options: `normal` (keyring), `bare` (plaintext), `simple` (XOR encoded)
   - Default: inherit from global setting or `normal`

### Behavior

- If user enters a password, show the storage mode selector
- If user leaves password empty, hide the selector
- For edit: show current mode (or "inherited: X" if using global)
- Indicator shows which mode is active for sessions with stored passwords

## CLI Changes

### `add` Command

```sh
se add --name office --host example.com --user bob --password --passwd-mode bare
se add --name office --host example.com --user bob --password --passwd-mode simple
```

New flag: `--passwd-mode <MODE>` with options `normal`, `bare`, `simple`. Default inherits from global or `normal`.

### `update` Command

```sh
se update --name office --password --passwd-mode simple
```

Same `--passwd-mode` flag for updating password storage mode.

### New `config` Command

```sh
se config set passwd_unsafe_mode bare
se config set passwd_unsafe_key my-secret-key
se config get passwd_unsafe_mode
```

Allows users to set/view global settings without manually editing JSON.

### `list` Command

No changes. Passwords never shown in list output.

## Error Handling

### Missing Key for "simple" Mode

- **On store**: Error: "No encryption key set. Set SSHER_UNSAFE_KEY env var or passwd_unsafe_key in config."
- **On retrieve**: Same error with suggestion to set the key

### Mode Changes

| From | To | Behavior |
|------|-----|----------|
| `normal` | `bare`/`simple` | Prompt to re-enter password (can't extract from keyring) |
| `bare`/`simple` | `normal` | Migrate to keyring if available, otherwise prompt |
| `bare` | `simple` | Re-encode with current key |
| `simple` | `bare` | Decode and store plaintext |

### Session Deletion

Delete password from:
1. Keyring (if `has_stored_password` is true)
2. `stored_password` field (clear on save)

### Import/Export

- **Export**: Never include `stored_password` field (security)
- **Import**: Ignore `stored_password` fields (don't import passwords from files)

### Security Notes

- **Config command**: When displaying `passwd_unsafe_key` via `se config get passwd_unsafe_key`, mask the value (show only confirmation that it's set, or show `****` with length indicator)
- **Mode transitions involving "simple"**: When changing from `bare` to `simple` or vice versa, require the key to be set. Raise error if key is unavailable.
- **Clear stored_password after migration**: When a session with `stored_password` is changed from `bare`/`simple` to `normal`, clear the `stored_password` field after successful keyring storage

## Data Model Changes

### New Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PasswdUnsafeMode {
    #[default]
    Normal,
    Bare,
    Simple,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    #[serde(default)]
    pub passwd_unsafe_mode: PasswdUnsafeMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passwd_unsafe_key: Option<String>,
    pub sessions: Vec<Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    // ... existing fields ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passwd_unsafe_mode: Option<PasswdUnsafeMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored_password: Option<String>,
}
```

### Store Trait Updates

```rust
pub trait SessionStore {
    // ... existing methods ...
    fn get_config(&self) -> Result<SessionStoreConfig>;
    fn set_config(&self, config: &SessionStoreConfig) -> Result<()>;
}
```

## Implementation Components

1. **`src/model.rs`**: Add `PasswdUnsafeMode` enum, update `Session` struct, add `SessionStore` wrapper struct
2. **`src/password.rs`**: Add unsafe storage functions (`store_unsafe_password`, `get_unsafe_password`, XOR encoding/decoding)
3. **`src/store/mod.rs`**: Update `JsonFileStore` to handle new format and migration
4. **`src/cli/mod.rs`**: Add `--passwd-mode` flag, new `config` command
5. **`src/ui/config.rs`**: Update form to include password mode selector
6. **`src/ui/mod.rs`**: Handle password mode in session creation/editing