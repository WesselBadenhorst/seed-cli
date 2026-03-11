use crate::util::{run_cmd, write_system_file};

pub fn switch_url(app_name: &str, new_url: &str) -> Result<(), String> {
    let nginx_available = format!("/etc/nginx/sites-available/{app_name}");
    let home = format!("/webapps/{app_name}");

    // 1. Update nginx server_name
    println!("→ updating nginx config...");
    let nginx_content = std::fs::read_to_string(&nginx_available)
        .map_err(|e| format!("failed to read nginx config: {e}"))?;

    let updated_nginx = update_server_name(&nginx_content, new_url);
    write_system_file(&nginx_available, &updated_nginx)?;

    // 2. Update .env ALLOWED_HOSTS
    println!("→ updating .env...");
    let env_path = find_env_file(&home)?;
    let env_content = std::fs::read_to_string(&env_path)
        .map_err(|e| format!("failed to read .env: {e}"))?;

    let updated_env = update_allowed_hosts(&env_content, new_url);
    write_system_file(&env_path, &updated_env)?;
    run_cmd("sudo", &["chown", &format!("{app_name}:{app_name}"), &env_path])?;

    // 3. Test and reload nginx
    run_cmd("sudo", &["nginx", "-t"])?;
    run_cmd("sudo", &["systemctl", "reload", "nginx"])?;

    // 4. Re-run certbot for new domain
    println!("→ setting up ssl for {new_url}...");
    run_cmd("sudo", &[
        "certbot", "--nginx", "-d", new_url,
        "--non-interactive", "--agree-tos", "--register-unsafely-without-email",
    ])?;

    // 5. Restart app to pick up new ALLOWED_HOSTS
    run_cmd("sudo", &["supervisorctl", "restart", app_name])?;

    println!("\n✓ {app_name} now serving at https://{new_url}");
    Ok(())
}

fn find_env_file(home: &str) -> Result<String, String> {
    // Walk home to find backend/.env
    for entry in std::fs::read_dir(home).map_err(|e| format!("cannot read {home}: {e}"))? {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            let env_path = path.join("backend").join(".env");
            if env_path.exists() {
                return Ok(env_path.to_string_lossy().to_string());
            }
        }
    }
    Err(format!("could not find backend/.env under {home}"))
}

fn update_server_name(nginx_content: &str, new_url: &str) -> String {
    nginx_content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("server_name ") {
                let indent = &line[..line.len() - line.trim_start().len()];
                format!("{indent}server_name {new_url};")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn update_allowed_hosts(env_content: &str, new_url: &str) -> String {
    env_content
        .lines()
        .map(|line| {
            if line.starts_with("ALLOWED_HOSTS=") {
                format!("ALLOWED_HOSTS={new_url}")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
