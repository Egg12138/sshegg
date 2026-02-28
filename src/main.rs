mod cli;
mod model;
mod password;
mod ssh;
mod store;
mod ui;

fn main() -> anyhow::Result<()> {
    cli::run()
}
