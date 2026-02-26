#compdef ssher

autoload -U is-at-least

_ssher() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'--store-path=[]:STORE_PATH:_files' \
'--ui-config=[]:UI_CONFIG:_files' \
'--cli-config=[]:CLI_CONFIG:_files' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_ssher_commands" \
"*::: :->ssher" \
&& ret=0
    case $state in
    (ssher)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ssher-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
'--name=[]:NAME:_default' \
'--host=[]:HOST:_default' \
'--user=[]:USER:_default' \
'--port=[]:PORT:_default' \
'--identity-file=[]:PATH:_files' \
'*--tag=[]:TAG:_default' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
'--name=[]:NAME:_default' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(tui)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(scp)
_arguments "${_arguments_options[@]}" : \
'--name=[]:NAME:_default' \
'--local=[]:PATH:_files' \
'--remote=[]:PATH:_files' \
'--direction=[]:DIRECTION:(to from)' \
'--recursive[]' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(completions)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':shell:(bash elvish fish powershell zsh)' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_ssher__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:ssher-help-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(remove)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tui)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(scp)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(completions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_ssher_commands] )) ||
_ssher_commands() {
    local commands; commands=(
'add:' \
'list:' \
'remove:' \
'tui:' \
'scp:' \
'completions:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ssher commands' commands "$@"
}
(( $+functions[_ssher__add_commands] )) ||
_ssher__add_commands() {
    local commands; commands=()
    _describe -t commands 'ssher add commands' commands "$@"
}
(( $+functions[_ssher__completions_commands] )) ||
_ssher__completions_commands() {
    local commands; commands=()
    _describe -t commands 'ssher completions commands' commands "$@"
}
(( $+functions[_ssher__help_commands] )) ||
_ssher__help_commands() {
    local commands; commands=(
'add:' \
'list:' \
'remove:' \
'tui:' \
'scp:' \
'completions:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'ssher help commands' commands "$@"
}
(( $+functions[_ssher__help__add_commands] )) ||
_ssher__help__add_commands() {
    local commands; commands=()
    _describe -t commands 'ssher help add commands' commands "$@"
}
(( $+functions[_ssher__help__completions_commands] )) ||
_ssher__help__completions_commands() {
    local commands; commands=()
    _describe -t commands 'ssher help completions commands' commands "$@"
}
(( $+functions[_ssher__help__help_commands] )) ||
_ssher__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'ssher help help commands' commands "$@"
}
(( $+functions[_ssher__help__list_commands] )) ||
_ssher__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'ssher help list commands' commands "$@"
}
(( $+functions[_ssher__help__remove_commands] )) ||
_ssher__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ssher help remove commands' commands "$@"
}
(( $+functions[_ssher__help__scp_commands] )) ||
_ssher__help__scp_commands() {
    local commands; commands=()
    _describe -t commands 'ssher help scp commands' commands "$@"
}
(( $+functions[_ssher__help__tui_commands] )) ||
_ssher__help__tui_commands() {
    local commands; commands=()
    _describe -t commands 'ssher help tui commands' commands "$@"
}
(( $+functions[_ssher__list_commands] )) ||
_ssher__list_commands() {
    local commands; commands=()
    _describe -t commands 'ssher list commands' commands "$@"
}
(( $+functions[_ssher__remove_commands] )) ||
_ssher__remove_commands() {
    local commands; commands=()
    _describe -t commands 'ssher remove commands' commands "$@"
}
(( $+functions[_ssher__scp_commands] )) ||
_ssher__scp_commands() {
    local commands; commands=()
    _describe -t commands 'ssher scp commands' commands "$@"
}
(( $+functions[_ssher__tui_commands] )) ||
_ssher__tui_commands() {
    local commands; commands=()
    _describe -t commands 'ssher tui commands' commands "$@"
}

if [ "$funcstack[1]" = "_ssher" ]; then
    _ssher "$@"
else
    compdef _ssher ssher
fi
