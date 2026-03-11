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
    /// Provision a new app
    New {
        /// Name of the app
        app_name: String,
        /// Git repository URL
        repo_url: String,
    },
    /// Update seed to the latest version
    Upgrade,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if !matches!(cli.command, Commands::Upgrade) {
        if let Err(missing) = preflight_check() {
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
            if let Err(e) = provision(&app_name, &repo_url) {
                eprintln!("provisioning failed: {e}");
                return ExitCode::FAILURE;
            }
        }
        Commands::Upgrade => {
            if let Err(e) = upgrade() {
                eprintln!("upgrade failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

fn provision(app_name: &str, _repo_url: &str) -> Result<(), String> {
    println!("provisioning {app_name}...");

    run_cmd("sudo", &["mkdir", "-p", "/webapps"])?;
    run_cmd("sudo", &[
        "useradd", "-m",
        "-d", &format!("/webapps/{app_name}"),
        "-s", "/usr/bin/zsh",
        app_name,
    ])?;

    run_cmd("tmux", &["new-session", "-d", "-s", app_name])?;
    run_cmd("tmux", &["split-window", "-h", "-t", app_name])?;

    // Switch to the new user in the left pane (su - does cd $HOME)
    run_cmd("tmux", &[
        "send-keys", "-t", &format!("{app_name}:0.0"),
        &format!("sudo su - {app_name}"), "Enter",
    ])?;

    run_cmd("tmux", &["attach-session", "-t", app_name])?;

    Ok(())
}

struct Dependency {
    name: &'static str,
    binary: &'static str,
    install_hint: &'static str,
}

const DEPENDENCIES: &[Dependency] = &[
    Dependency {
        name: "tmux",
        binary: "tmux",
        install_hint: "sudo apt install tmux",
    },
    Dependency {
        name: "neovim",
        binary: "nvim",
        install_hint: "sudo apt install neovim",
    },
    Dependency {
        name: "nginx",
        binary: "nginx",
        install_hint: "sudo apt install nginx",
    },
    Dependency {
        name: "supervisor",
        binary: "supervisord",
        install_hint: "sudo apt install supervisor",
    },
    Dependency {
        name: "postgresql",
        binary: "psql",
        install_hint: "sudo apt install postgresql",
    },
];

fn preflight_check() -> Result<(), Vec<(&'static str, &'static str)>> {
    let missing: Vec<_> = DEPENDENCIES
        .iter()
        .filter(|dep| !command_exists(dep.binary))
        .map(|dep| (dep.name, dep.install_hint))
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
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
