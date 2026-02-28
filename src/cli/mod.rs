mod output;
mod theme;

use crate::model::Session;
use crate::store::{JsonFileStore, resolve_store_path};
use crate::ui;
use anyhow::{Context, Result, anyhow};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use std::io;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "ssher",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_COMMIT_HASH"), ")"),
    about = "Manage SSH sessions"
)]
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
    Export(ExportArgs),
    Import(ImportArgs),
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
struct ExportArgs {
    #[arg(long, value_enum, default_value = "json")]
    format: ExportFormat,
    #[arg(long, value_name = "PATH")]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct ImportArgs {
    #[arg(long, value_enum, default_value = "json")]
    format: ImportFormat,
    #[arg(long, value_name = "PATH")]
    input: PathBuf,
    #[arg(
        long,
        short = 'f',
        help = "Force override existing sessions with same name"
    )]
    force: bool,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum ExportFormat {
    Json,
    Csv,
    SshConfig,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum ImportFormat {
    Json,
    SshConfig,
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
                Some(Commands::Export(args)) => export_sessions(&store, args),
                Some(Commands::Import(args)) => import_sessions(&store, args),
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

fn export_sessions(store: &JsonFileStore, args: ExportArgs) -> Result<()> {
    let sessions = store.list()?;
    let output = match args.format {
        ExportFormat::Json => export_to_json(&sessions)?,
        ExportFormat::Csv => export_to_csv(&sessions),
        ExportFormat::SshConfig => export_to_ssh_config(&sessions),
    };

    if let Some(path) = args.output {
        std::fs::write(&path, output)
            .with_context(|| format!("failed to write to {}", path.display()))?;
        println!("Exported {} sessions to {}", sessions.len(), path.display());
    } else {
        print!("{}", output);
    }
    Ok(())
}

fn export_to_json(sessions: &[Session]) -> Result<String> {
    serde_json::to_string_pretty(sessions).context("failed to serialize sessions to JSON")
}

fn export_to_csv(sessions: &[Session]) -> String {
    let mut csv = String::new();
    csv.push_str("name,host,user,port,identity_file,tags\n");
    for session in sessions {
        let identity = session
            .identity_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let tags = session.tags.join(";");
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            escape_csv(&session.name),
            escape_csv(&session.host),
            escape_csv(&session.user),
            session.port,
            escape_csv(&identity),
            escape_csv(&tags)
        ));
    }
    csv
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn export_to_ssh_config(sessions: &[Session]) -> String {
    let mut config = String::new();
    config.push_str("# Generated by ssher\n\n");
    for session in sessions {
        config.push_str(&format!("Host {}\n", session.name));
        config.push_str(&format!("    HostName {}\n", session.host));
        config.push_str(&format!("    User {}\n", session.user));
        config.push_str(&format!("    Port {}\n", session.port));
        if let Some(identity) = &session.identity_file {
            config.push_str(&format!("    IdentityFile {}\n", identity.display()));
        }
        if !session.tags.is_empty() {
            config.push_str(&format!("    # Tags: {}\n", session.tags.join(", ")));
        }
        config.push('\n');
    }
    config
}

fn import_sessions(store: &JsonFileStore, args: ImportArgs) -> Result<()> {
    let input_content = std::fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;

    let imported_sessions = match args.format {
        ImportFormat::Json => import_from_json(&input_content)?,
        ImportFormat::SshConfig => import_from_ssh_config(&input_content)?,
    };

    let existing_sessions = store.list()?;
    let existing_names: std::collections::HashSet<String> =
        existing_sessions.iter().map(|s| s.name.clone()).collect();

    if args.force {
        // Force mode: override existing sessions
        for session in &imported_sessions {
            if existing_names.contains(&session.name) {
                println!("Overriding existing session: {}", session.name);
            }
        }
        for session in &imported_sessions {
            store.add(session.clone())?;
        }
        println!("Imported {} sessions", imported_sessions.len());
    } else {
        // Interactive mode: handle conflicts
        let mut conflicts: Vec<Session> = Vec::new();
        let mut to_import: Vec<Session> = Vec::new();

        for session in imported_sessions {
            if existing_names.contains(&session.name) {
                conflicts.push(session);
            } else {
                to_import.push(session);
            }
        }

        // Import non-conflicting sessions
        for session in &to_import {
            store.add(session.clone())?;
            println!("Imported: {}", session.name);
        }

        // Handle conflicts
        if !conflicts.is_empty() {
            println!("\n{} conflict(s) found:", conflicts.len());
            for (i, session) in conflicts.iter().enumerate() {
                println!(
                    "  {}. {} (host: {}, user: {})",
                    i + 1,
                    session.name,
                    session.host,
                    session.user
                );
            }
            println!("\nOptions:");
            println!("  [o] Override all conflicts");
            println!("  [s] Skip all conflicts");
            println!("  [i] Interactively choose for each");
            println!("  [q] Quit without importing conflicts");

            let mut choice = String::new();
            std::io::stdin().read_line(&mut choice)?;
            choice = choice.trim().to_lowercase();

            match choice.as_str() {
                "o" => {
                    for session in &conflicts {
                        store.add(session.clone())?;
                        println!("Overridden: {}", session.name);
                    }
                }
                "s" => {
                    println!("Skipped {} conflict(s)", conflicts.len());
                }
                "i" => {
                    for session in &conflicts {
                        print!(
                            "Override '{}' (host: {}, user: {})? [y/N/s/q] ",
                            session.name, session.host, session.user
                        );
                        std::io::Write::flush(&mut std::io::stdout())?;
                        let mut response = String::new();
                        std::io::stdin().read_line(&mut response)?;
                        response = response.trim().to_lowercase();

                        match response.as_str() {
                            "y" | "yes" => {
                                store.add(session.clone())?;
                                println!("Overridden: {}", session.name);
                            }
                            "s" => {
                                println!("Skipped remaining conflicts");
                                break;
                            }
                            "q" => {
                                println!("Aborted");
                                break;
                            }
                            _ => {
                                println!("Skipped: {}", session.name);
                            }
                        }
                    }
                }
                "q" => {
                    println!("Aborted importing conflicts");
                }
                _ => {
                    println!("Aborted importing conflicts");
                }
            }
        }
    }

    Ok(())
}

fn import_from_json(content: &str) -> Result<Vec<Session>> {
    serde_json::from_str(content).context("failed to parse JSON")
}

fn import_from_ssh_config(content: &str) -> Result<Vec<Session>> {
    let mut sessions = Vec::new();
    let mut current_host: Option<String> = None;
    let mut current_user = "root".to_string();
    let mut current_hostname: Option<String> = None;
    let mut current_port = 22u16;
    let mut current_identity: Option<PathBuf> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }

        let keyword = parts[0].to_lowercase();
        let value = parts[1].trim();

        match keyword.as_str() {
            "host" => {
                // Save previous host if exists
                if let Some(name) = current_host.take()
                    && let Some(hostname) = current_hostname.take()
                {
                    sessions.push(Session {
                        name,
                        host: hostname,
                        user: current_user.clone(),
                        port: current_port,
                        identity_file: current_identity.take(),
                        tags: vec![],
                        last_connected_at: None,
                    });
                }
                current_host = Some(value.to_string());
                current_user = "root".to_string();
                current_port = 22;
                current_identity = None;
            }
            "user" => {
                current_user = value.to_string();
            }
            "hostname" => {
                current_hostname = Some(value.to_string());
            }
            "port" => {
                current_port = value.parse().unwrap_or(22);
            }
            "identityfile" => {
                current_identity = Some(PathBuf::from(value));
            }
            _ => {}
        }
    }

    // Save last host
    if let Some(name) = current_host.take()
        && let Some(hostname) = current_hostname.take()
    {
        sessions.push(Session {
            name,
            host: hostname,
            user: current_user.clone(),
            port: current_port,
            identity_file: current_identity.take(),
            tags: vec![],
            last_connected_at: None,
        });
    }

    Ok(sessions)
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
