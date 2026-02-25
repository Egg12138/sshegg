mod cli;
mod model;
mod store;
mod ui;

fn main() -> anyhow::Result<()> {
    cli::run()
}
