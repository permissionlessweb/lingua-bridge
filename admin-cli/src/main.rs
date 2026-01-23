mod tui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "linguabridge-admin")]
#[command(about = "Admin CLI for LinguaBridge")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the Terminal User Interface
    Tui,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Tui => {
            tui::run_tui().await
        }
    }
}
