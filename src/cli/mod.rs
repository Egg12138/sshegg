mod output;
mod theme;

use crate::model::Session;
use crate::store::{JsonFileStore, resolve_store_path};
use crate::ui;
use anyhow::{Context, Result, anyhow};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::io;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "ssher", version, about = "Manage SSH sessions")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long, env = "SSHER_STORE")]
    store_path: Option<PathBuf>,
    #[arg(long, env = "SSHER_UI_CONFIG")]
    ui_config: Option<PathBuf>,
    #[arg(long, env = "SSHER_CLI_CONFIG")]
    cli_config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Add(AddArgs),
    Update(UpdateArgs),
    List,
    Remove(RemoveArgs),
    Tui,
    Scp(ScpArgs),
    Completions(CompletionsArgs),
}

#[derive(Args)]
struct AddArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    host: String,
    #[arg(long)]
    user: String,
    #[arg(long, default_value_t = 22)]
    port: u16,
    #[arg(long, value_name = "PATH")]
    identity_file: Option<PathBuf>,
    #[arg(
        long = "tag",
        alias = "tags",
        value_name = "TAG",
        value_delimiter = ','
    )]
    tags: Vec<String>,
}

#[derive(Args)]
struct RemoveArgs {
    #[arg(long)]
    name: String,
}

#[derive(Args)]
struct UpdateArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    user: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long, value_name = "PATH")]
    identity_file: Option<PathBuf>,
    #[arg(
        long = "tag",
        alias = "tags",
        value_name = "TAG",
        value_delimiter = ','
    )]
    tags: Vec<String>,
}

#[derive(Args)]
struct ScpArgs {
    #[arg(long)]
    name: String,
    #[arg(long, value_name = "PATH")]
    local: PathBuf,
    #[arg(long, value_name = "PATH")]
    remote: PathBuf,
    #[arg(long, value_enum, default_value = "to")]
    direction: ScpDirection,
    #[arg(long)]
    recursive: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ScpDirection {
    To,
    From,
}

#[derive(Args)]
struct CompletionsArgs {
    #[arg(value_enum)]
    shell: Shell,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Completions(args)) => {
            generate_completions(args.shell);
            Ok(())
        }
        _ => {
            let store_path = resolve_store_path(cli.store_path)?;
            let store = JsonFileStore::new(store_path);

            match cli.command {
                Some(Commands::Add(args)) => add_session(&store, args),
                Some(Commands::Update(args)) => update_session(&store, args),
                Some(Commands::List) => list_sessions(&store, cli.cli_config),
                Some(Commands::Remove(args)) => remove_session(&store, &args.name),
                Some(Commands::Tui) | None => {
                    let ui_config = ui::load_ui_config(cli.ui_config)?;
                    run_tui(&store, &ui_config)
                }
                Some(Commands::Scp(args)) => run_scp(&store, args),
                Some(Commands::Completions(_)) => unreachable!(),
            }
        }
    }
}

fn add_session(store: &JsonFileStore, args: AddArgs) -> Result<()> {
    let session = Session {
        name: args.name,
        host: args.host,
        user: args.user,
        port: args.port,
        identity_file: args.identity_file,
        tags: normalize_tags(args.tags),
        last_connected_at: None,
    };
    store.add(session.clone())?;
    println!("Added session: {}", session.name);
    Ok(())
}

fn list_sessions(store: &JsonFileStore, cli_config: Option<PathBuf>) -> Result<()> {
    let sessions = store.list()?;
    let theme = theme::load_cli_theme(cli_config)?;
    output::print_sessions(&sessions, &theme);
    Ok(())
}

fn remove_session(store: &JsonFileStore, name: &str) -> Result<()> {
    store.remove(name)?;
    println!("Removed session: {}", name);
    Ok(())
}

fn update_session(store: &JsonFileStore, args: UpdateArgs) -> Result<()> {
    let mut sessions = store.list()?;
    let session = sessions
        .iter_mut()
        .find(|s| s.name == args.name)
        .ok_or_else(|| anyhow!("session '{}' not found", args.name))?;

    if let Some(host) = args.host {
        session.host = host;
    }
    if let Some(user) = args.user {
        session.user = user;
    }
    if let Some(port) = args.port {
        session.port = port;
    }
    if args.identity_file.is_some() {
        session.identity_file = args.identity_file;
    }
    if !args.tags.is_empty() {
        session.tags = normalize_tags(args.tags);
    }

    store.update(session.clone())?;
    println!("Updated session: {}", session.name);
    Ok(())
}

fn run_tui(store: &JsonFileStore, ui_config: &ui::UiConfig) -> Result<()> {
    let selection = ui::run_tui(store, ui_config)?;
    if let Some(session) = selection {
        run_ssh(&session)?;
        store.touch_last_connected(&session.name, now_epoch_seconds())?;
    }
    Ok(())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn run_ssh(session: &Session) -> Result<()> {
    let mut command = Command::new("ssh");
    if let Some(identity) = &session.identity_file {
        command.arg("-i").arg(identity);
    }
    command
        .arg("-p")
        .arg(session.port.to_string())
        .arg(session.target());
    let status = command.status().context("failed to execute ssh")?;
    if !status.success() {
        return Err(anyhow!("ssh exited with status {}", status));
    }
    Ok(())
}

fn run_scp(store: &JsonFileStore, args: ScpArgs) -> Result<()> {
    let session = store
        .list()?
        .into_iter()
        .find(|session| session.name == args.name)
        .ok_or_else(|| anyhow!("session '{}' not found", args.name))?;

    let mut command = Command::new("scp");
    if args.recursive {
        command.arg("-r");
    }
    if let Some(identity) = &session.identity_file {
        command.arg("-i").arg(identity);
    }
    command.arg("-P").arg(session.port.to_string());

    let remote_target = format!(
        "{}@{}:{}",
        session.user,
        session.host,
        args.remote.display()
    );
    match args.direction {
        ScpDirection::To => {
            command.arg(args.local).arg(remote_target);
        }
        ScpDirection::From => {
            command.arg(remote_target).arg(args.local);
        }
    }

    let status = command.status().context("failed to execute scp")?;
    if !status.success() {
        return Err(anyhow!("scp exited with status {}", status));
    }
    store.touch_last_connected(&session.name, now_epoch_seconds())?;
    Ok(())
}

fn now_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "ssher", &mut io::stdout());
}
