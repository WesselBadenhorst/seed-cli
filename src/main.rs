use clap::{Parser, Subcommand};
use std::process::{Command, ExitCode};

const REPO_URL: &str = "https://github.com/WesselBadenhorst/seed-cli.git";

#[derive(Parser)]
#[command(name = "seed", about = "A personal provisioning tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Update seed to the latest version
    Upgrade,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Upgrade => {
            if let Err(e) = upgrade() {
                eprintln!("upgrade failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn upgrade() -> Result<(), String> {
    let tmp_dir = std::env::temp_dir().join("seed-cli-upgrade");

    // Clean up any previous failed attempt
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).map_err(|e| format!("failed to clean tmp dir: {e}"))?;
    }

    println!("cloning seed-cli...");
    run_cmd("git", &["clone", "--depth", "1", REPO_URL, tmp_dir.to_str().unwrap()])?;

    println!("building and installing...");
    run_cmd("cargo", &["install", "--path", tmp_dir.to_str().unwrap()])?;

    // Clean up
    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!("seed upgraded successfully");
    Ok(())
}

fn run_cmd(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {program}: {e}"))?;

    if !status.success() {
        return Err(format!("{program} exited with {status}"));
    }
    Ok(())
}
