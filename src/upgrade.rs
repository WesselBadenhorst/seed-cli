use crate::util::run_cmd;

const REPO_URL: &str = "https://github.com/WesselBadenhorst/seed-cli.git";

pub fn upgrade() -> Result<(), String> {
    let tmp_dir = std::env::temp_dir().join("seed-cli-upgrade");

    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).map_err(|e| format!("failed to clean tmp dir: {e}"))?;
    }

    println!("cloning seed-cli...");
    run_cmd("git", &["clone", "--depth", "1", REPO_URL, tmp_dir.to_str().unwrap()])?;

    println!("building and installing...");
    run_cmd("cargo", &["install", "--path", tmp_dir.to_str().unwrap()])?;

    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!("seed upgraded successfully");
    Ok(())
}
