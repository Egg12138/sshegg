# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_ssher_global_optspecs
	string join \n store-path= ui-config= cli-config= h/help V/version
end

function __fish_ssher_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_ssher_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_ssher_using_subcommand
	set -l cmd (__fish_ssher_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c ssher -n "__fish_ssher_needs_command" -l store-path -r -F
complete -c ssher -n "__fish_ssher_needs_command" -l ui-config -r -F
complete -c ssher -n "__fish_ssher_needs_command" -l cli-config -r -F
complete -c ssher -n "__fish_ssher_needs_command" -s h -l help -d 'Print help'
complete -c ssher -n "__fish_ssher_needs_command" -s V -l version -d 'Print version'
complete -c ssher -n "__fish_ssher_needs_command" -f -a "add"
complete -c ssher -n "__fish_ssher_needs_command" -f -a "list"
complete -c ssher -n "__fish_ssher_needs_command" -f -a "remove"
complete -c ssher -n "__fish_ssher_needs_command" -f -a "tui"
complete -c ssher -n "__fish_ssher_needs_command" -f -a "scp"
complete -c ssher -n "__fish_ssher_needs_command" -f -a "completions"
complete -c ssher -n "__fish_ssher_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c ssher -n "__fish_ssher_using_subcommand add" -l name -r
complete -c ssher -n "__fish_ssher_using_subcommand add" -l host -r
complete -c ssher -n "__fish_ssher_using_subcommand add" -l user -r
complete -c ssher -n "__fish_ssher_using_subcommand add" -l port -r
complete -c ssher -n "__fish_ssher_using_subcommand add" -l identity-file -r -F
complete -c ssher -n "__fish_ssher_using_subcommand add" -l tag -r
complete -c ssher -n "__fish_ssher_using_subcommand add" -s h -l help -d 'Print help'
complete -c ssher -n "__fish_ssher_using_subcommand list" -s h -l help -d 'Print help'
complete -c ssher -n "__fish_ssher_using_subcommand remove" -l name -r
complete -c ssher -n "__fish_ssher_using_subcommand remove" -s h -l help -d 'Print help'
complete -c ssher -n "__fish_ssher_using_subcommand tui" -s h -l help -d 'Print help'
complete -c ssher -n "__fish_ssher_using_subcommand scp" -l name -r
complete -c ssher -n "__fish_ssher_using_subcommand scp" -l local -r -F
complete -c ssher -n "__fish_ssher_using_subcommand scp" -l remote -r -F
complete -c ssher -n "__fish_ssher_using_subcommand scp" -l direction -r -f -a "to\t''
from\t''"
complete -c ssher -n "__fish_ssher_using_subcommand scp" -l recursive
complete -c ssher -n "__fish_ssher_using_subcommand scp" -s h -l help -d 'Print help'
complete -c ssher -n "__fish_ssher_using_subcommand completions" -s h -l help -d 'Print help'
complete -c ssher -n "__fish_ssher_using_subcommand help; and not __fish_seen_subcommand_from add list remove tui scp completions help" -f -a "add"
complete -c ssher -n "__fish_ssher_using_subcommand help; and not __fish_seen_subcommand_from add list remove tui scp completions help" -f -a "list"
complete -c ssher -n "__fish_ssher_using_subcommand help; and not __fish_seen_subcommand_from add list remove tui scp completions help" -f -a "remove"
complete -c ssher -n "__fish_ssher_using_subcommand help; and not __fish_seen_subcommand_from add list remove tui scp completions help" -f -a "tui"
complete -c ssher -n "__fish_ssher_using_subcommand help; and not __fish_seen_subcommand_from add list remove tui scp completions help" -f -a "scp"
complete -c ssher -n "__fish_ssher_using_subcommand help; and not __fish_seen_subcommand_from add list remove tui scp completions help" -f -a "completions"
complete -c ssher -n "__fish_ssher_using_subcommand help; and not __fish_seen_subcommand_from add list remove tui scp completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
