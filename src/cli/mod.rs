mod output;

use crate::model::Session;
use crate::store::{JsonFileStore, resolve_store_path};
use crate::ui;
use anyhow::{Context, Result, anyhow};
use clap::{Args, Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "ssher", version, about = "Manage SSH sessions")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long, env = "SSHER_STORE")]
    store_path: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Add(AddArgs),
    List,
    Remove(RemoveArgs),
    Tui,
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
}

#[derive(Args)]
struct RemoveArgs {
    #[arg(long)]
    name: String,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let store_path = resolve_store_path(cli.store_path)?;
    let store = JsonFileStore::new(store_path);

    match cli.command {
        Some(Commands::Add(args)) => add_session(&store, args),
        Some(Commands::List) => list_sessions(&store),
        Some(Commands::Remove(args)) => remove_session(&store, &args.name),
        Some(Commands::Tui) | None => run_tui(&store),
    }
}

fn add_session(store: &JsonFileStore, args: AddArgs) -> Result<()> {
    let session = Session {
        name: args.name,
        host: args.host,
        user: args.user,
        port: args.port,
        identity_file: args.identity_file,
    };
    store.add(session.clone())?;
    println!("Added session: {}", session.name);
    Ok(())
}

fn list_sessions(store: &JsonFileStore) -> Result<()> {
    let sessions = store.list()?;
    output::print_sessions(&sessions);
    Ok(())
}

fn remove_session(store: &JsonFileStore, name: &str) -> Result<()> {
    store.remove(name)?;
    println!("Removed session: {}", name);
    Ok(())
}

fn run_tui(store: &JsonFileStore) -> Result<()> {
    let sessions = store.list()?;
    if sessions.is_empty() {
        println!("No sessions found. Let's create one.");
        interactive_add_session(store)?;
    }
    let sessions = store.list()?;
    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }
    let selection = ui::run_tui(&sessions)?;
    if let Some(session) = selection {
        run_ssh(&session)?;
    }
    Ok(())
}

fn interactive_add_session(store: &JsonFileStore) -> Result<Session> {
    let name = prompt_required("Name")?;
    let host = prompt_required("Host")?;
    let user = prompt_required("User")?;
    let port = prompt_port("Port", 22)?;
    let identity_file = prompt_optional_path("Identity file path (optional)")?;

    let session = Session {
        name,
        host,
        user,
        port,
        identity_file,
    };
    store.add(session.clone())?;
    println!("Added session: {}", session.name);
    Ok(session)
}

fn prompt_required(label: &str) -> Result<String> {
    loop {
        let value = prompt(label)?;
        if !value.is_empty() {
            return Ok(value);
        }
        println!("{} cannot be empty.", label);
    }
}

fn prompt_port(label: &str, default_port: u16) -> Result<u16> {
    loop {
        let value = prompt(&format!("{} [{}]", label, default_port))?;
        if value.is_empty() {
            return Ok(default_port);
        }
        match value.parse::<u16>() {
            Ok(port) => return Ok(port),
            Err(_) => println!("{} must be a valid port number.", label),
        }
    }
}

fn prompt_optional_path(label: &str) -> Result<Option<PathBuf>> {
    let value = prompt(label)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(value)))
    }
}

fn prompt(label: &str) -> Result<String> {
    print!("{}: ", label);
    io::stdout().flush().context("unable to flush prompt")?;
    let mut input = String::new();
    let read = io::stdin()
        .read_line(&mut input)
        .context("unable to read input")?;
    if read == 0 {
        return Err(anyhow!("input stream closed"));
    }
    Ok(input.trim().to_string())
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
