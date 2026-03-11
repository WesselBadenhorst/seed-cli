use std::process::{Command, Stdio};

pub fn run_cmd(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {program}: {e}"))?;

    if !status.success() {
        return Err(format!("{program} exited with {status}"));
    }
    Ok(())
}

pub fn run_cmd_as_user(user: &str, cmd: &str) -> Result<(), String> {
    run_cmd("sudo", &["-u", user, "bash", "-c", cmd])
}

pub fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn run_cmd_output(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {program}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{program} failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn user_exists(name: &str) -> bool {
    Command::new("id")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub fn path_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

pub fn pg_role_exists(name: &str) -> bool {
    Command::new("sudo")
        .args(["-u", "postgres", "psql", "-tAc",
            &format!("SELECT 1 FROM pg_roles WHERE rolname='{name}'")])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
        .unwrap_or(false)
}

pub fn pg_db_exists(name: &str) -> bool {
    Command::new("sudo")
        .args(["-u", "postgres", "psql", "-tAc",
            &format!("SELECT 1 FROM pg_database WHERE datname='{name}'")])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
        .unwrap_or(false)
}

pub fn prompt(message: &str) -> Result<String, String> {
    eprint!("{message}: ");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return Err("input cannot be empty".to_string());
    }
    Ok(trimmed)
}

pub fn wait_for_enter(message: &str) -> Result<(), String> {
    eprint!("{message}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    Ok(())
}

pub fn generate_secret(length: usize) -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*(-_=+)";
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn generate_password(length: usize) -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub fn repo_name_from_url(repo_url: &str) -> String {
    repo_url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git")
        .to_string()
}

pub fn write_system_file(path: &str, content: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join("seed-tmp-file");
    std::fs::write(&tmp, content)
        .map_err(|e| format!("failed to write temp file: {e}"))?;
    run_cmd("sudo", &["cp", tmp.to_str().unwrap(), path])?;
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}
