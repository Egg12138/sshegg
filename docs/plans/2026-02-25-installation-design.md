# ssher Installation Design

## Overview

Create a local installation script (`scripts/install.sh`) for building and installing ssher from source. The script provides a user-friendly, step-by-step installation process with clear output and optional shell completions.

## Directory Structure

```
ssher/
├── scripts/
│   ├── install.sh      # Main installation script
│   └── completions/    # Shell completion templates
│       ├── ssher.bash
│       ├── ssher.zsh
│       └── ssher.fish
└── assets/
    ├── ui.sample.json  # Already exists
    └── cli.sample.json # Already exists
```

## Script Flow

1. **Parse CLI arguments**
   - `--help, -h` - Show formatted help
   - `--prefix PATH` - Custom install location (default: `~/.local/bin`)
   - `--from-source` - Force build from source
   - `--no-completions` - Skip shell completions

2. **Check dependencies**
   - Verify rust/cargo is installed
   - Verify ssh command is available
   - Print clear warnings if missing

3. **Build the binary**
   - Run `cargo build --release`
   - Show clear progress messages

4. **Install the binary**
   - Create target directory if needed
   - Copy `target/release/ssher`
   - Mark as executable

5. **Setup configuration**
   - Create `~/.config/ssher/`
   - Copy sample configs only if they don't exist

6. **Install completions**
   - Detect active shells (bash/zsh/fish)
   - Copy to appropriate locations

7. **Print summary**
   - Binary location
   - Config location
   - Usage examples

## User Experience

### Installation

```bash
git clone https://github.com/user/ssher.git
cd ssher
./scripts/install.sh
```

### Output Style

- Step-by-step messages with icons (✓ ✓ ✗)
- Colored output (green=success, yellow=warning, red=error, blue=info)
- Clear section headers
- Brief, friendly messages

Example:
```
==> Installing ssher

==> Checking dependencies...
  ✓ Rust toolchain found
  ✓ ssh command available

==> Building binary...
  Building release binary...
  ✓ Build complete

==> Installing to /home/user/.local/bin...
  ✓ Binary installed

==> Setting up configuration...
  ✓ Config directory: ~/.config/ssher

==> Installing shell completions...
  ✓ bash completions installed
  ✓ zsh completions installed

==> Installation complete!
```

## Error Handling

| Scenario | Action |
|----------|--------|
| Missing Rust | Suggest rustup installation |
| Missing SSH | Suggest openssh-client install (OS-specific) |
| Permission denied | Suggest `--prefix` for user-writable location |
| Build failure | Show cargo errors, suggest opening issue |
| Directory failure | Clear error message about what failed |

## Shell Completions

Generated using clap's derive feature, providing:
- Subcommand completion (add, list, remove, tui, scp)
- Flag/option completion
- Tag completion from existing sessions

Completion locations:
- bash: `~/.local/share/bash-completion/completions/ssher`
- zsh: `${ZDOTDIR:-~}/.zfunc/_ssher`
- fish: `~/.config/fish/completions/ssher.fish`
