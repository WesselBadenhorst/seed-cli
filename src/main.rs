mod preflight;
mod provision;
mod remove;
mod switch_url;
mod templates;
mod upgrade;
mod util;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "seed", about = "A personal provisioning tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Provision a new app
    New {
        /// Name of the app
        app_name: String,
        /// Git repository URL
        repo_url: String,
    },
    /// Remove an app and all its resources
    Rm {
        /// Name of the app to remove
        app_name: String,
    },
    /// Change the domain for an app
    SwitchUrl {
        /// Name of the app
        app_name: String,
        /// New domain
        new_url: String,
    },
    /// Update seed to the latest version
    Upgrade,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if !matches!(cli.command, Commands::Upgrade) {
        if let Err(missing) = preflight::preflight_check() {
            eprintln!("seed: missing required dependencies:\n");
            for (name, hint) in &missing {
                eprintln!("  ✗ {name}");
                eprintln!("    → {hint}\n");
            }
            eprintln!("install the above and try again.");
            return ExitCode::FAILURE;
        }
    }

    match cli.command {
        Commands::New { app_name, repo_url } => {
            if let Err(e) = provision::provision(&app_name, &repo_url) {
                eprintln!("provisioning failed: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Rm { app_name } => {
            if let Err(e) = remove::remove(&app_name) {
                eprintln!("remove failed: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::SwitchUrl { app_name, new_url } => {
            if let Err(e) = switch_url::switch_url(&app_name, &new_url) {
                eprintln!("switch-url failed: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Upgrade => {
            if let Err(e) = upgrade::upgrade() {
                eprintln!("upgrade failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}
