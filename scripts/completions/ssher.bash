# ssher.bash - Placeholder for bash completions
# Generated via: cargo run -- completions bash
_ssher_completion() {
    local cur prev words cword
    _init_completion || return
    # Will be populated by clap-generated completions
}
complete -F _ssher_completion ssher
