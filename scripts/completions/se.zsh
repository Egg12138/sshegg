#compdef se

autoload -U is-at-least

_se() {
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
":: :_se_commands" \
"*::: :->se" \
&& ret=0
    case $state in
    (se)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:se-command-$line[1]:"
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
":: :_se__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:se-help-command-$line[1]:"
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

(( $+functions[_se_commands] )) ||
_se_commands() {
    local commands; commands=(
'add:' \
'list:' \
'remove:' \
'tui:' \
'scp:' \
'completions:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'se commands' commands "$@"
}
(( $+functions[_se__add_commands] )) ||
_se__add_commands() {
    local commands; commands=()
    _describe -t commands 'se add commands' commands "$@"
}
(( $+functions[_se__completions_commands] )) ||
_se__completions_commands() {
    local commands; commands=()
    _describe -t commands 'se completions commands' commands "$@"
}
(( $+functions[_se__help_commands] )) ||
_se__help_commands() {
    local commands; commands=(
'add:' \
'list:' \
'remove:' \
'tui:' \
'scp:' \
'completions:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'se help commands' commands "$@"
}
(( $+functions[_se__help__add_commands] )) ||
_se__help__add_commands() {
    local commands; commands=()
    _describe -t commands 'se help add commands' commands "$@"
}
(( $+functions[_se__help__completions_commands] )) ||
_se__help__completions_commands() {
    local commands; commands=()
    _describe -t commands 'se help completions commands' commands "$@"
}
(( $+functions[_se__help__help_commands] )) ||
_se__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'se help help commands' commands "$@"
}
(( $+functions[_se__help__list_commands] )) ||
_se__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'se help list commands' commands "$@"
}
(( $+functions[_se__help__remove_commands] )) ||
_se__help__remove_commands() {
    local commands; commands=()
    _describe -t commands 'se help remove commands' commands "$@"
}
(( $+functions[_se__help__scp_commands] )) ||
_se__help__scp_commands() {
    local commands; commands=()
    _describe -t commands 'se help scp commands' commands "$@"
}
(( $+functions[_se__help__tui_commands] )) ||
_se__help__tui_commands() {
    local commands; commands=()
    _describe -t commands 'se help tui commands' commands "$@"
}
(( $+functions[_se__list_commands] )) ||
_se__list_commands() {
    local commands; commands=()
    _describe -t commands 'se list commands' commands "$@"
}
(( $+functions[_se__remove_commands] )) ||
_se__remove_commands() {
    local commands; commands=()
    _describe -t commands 'se remove commands' commands "$@"
}
(( $+functions[_se__scp_commands] )) ||
_se__scp_commands() {
    local commands; commands=()
    _describe -t commands 'se scp commands' commands "$@"
}
(( $+functions[_se__tui_commands] )) ||
_se__tui_commands() {
    local commands; commands=()
    _describe -t commands 'se tui commands' commands "$@"
}

if [ "$funcstack[1]" = "_se" ]; then
    _se "$@"
else
    compdef _se se
fi
