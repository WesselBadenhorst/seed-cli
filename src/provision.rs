use crate::templates;
use crate::util::{
    generate_password, generate_secret, path_exists, pg_db_exists, pg_role_exists, prompt,
    repo_name_from_url, run_cmd, run_cmd_as_user, run_cmd_output, user_exists, wait_for_enter,
    write_system_file,
};

pub fn provision(app_name: &str, repo_url: &str) -> Result<(), String> {
    let domain = prompt("app url (domain)")?;
    let repo_name = repo_name_from_url(repo_url);
    let home = format!("/webapps/{app_name}");
    let repo_dir = format!("{home}/{repo_name}");
    let backend_dir = format!("{repo_dir}/backend");
    let frontend_dir = format!("{repo_dir}/frontend");
    let pane = format!("{app_name}:0.0");

    println!("provisioning {app_name}...\n");

    // 1. Create system user
    if user_exists(app_name) {
        println!("→ user {app_name} already exists, skipping...");
    } else {
        println!("→ creating user {app_name}...");
        run_cmd("sudo", &["mkdir", "-p", "/webapps"])?;
        run_cmd("sudo", &[
            "useradd", "-m",
            "-d", &home,
            "-s", "/usr/bin/zsh",
            app_name,
        ])?;
    }

    // 2. Create run/ and logs/ directories
    println!("→ creating directories...");
    run_cmd("sudo", &["-u", app_name, "mkdir", "-p", &format!("{home}/run")])?;
    run_cmd("sudo", &["-u", app_name, "mkdir", "-p", &format!("{home}/logs")])?;

    // 3. Generate SSH key
    let key_path = format!("{home}/.ssh/id_ed25519");
    if path_exists(&format!("{key_path}.pub")) {
        let pub_key = run_cmd_output("sudo", &["cat", &format!("{key_path}.pub")])?;
        println!("→ ssh key already exists:");
        println!("{pub_key}\n");
    } else {
        println!("→ generating ssh key...");
        let ssh_dir = format!("{home}/.ssh");
        run_cmd_as_user(app_name, &format!(
            "mkdir -p {ssh_dir} && chmod 700 {ssh_dir} && ssh-keygen -t ed25519 -f {key_path} -N '' -C '{app_name}@seed'"
        ))?;
        let pub_key = run_cmd_output("sudo", &["cat", &format!("{key_path}.pub")])?;
        println!("\n╭─────────────────────────────────────────╮");
        println!("│  Add this deploy key to your repository  │");
        println!("╰─────────────────────────────────────────╯\n");
        println!("{pub_key}\n");
    }

    // 4. Start tmux session and switch to app user in left pane
    println!("→ starting tmux session...");
    run_cmd("tmux", &["new-session", "-d", "-s", app_name])?;
    run_cmd("tmux", &["split-window", "-h", "-t", app_name])?;
    tmux_send(app_name, &pane, &format!("sudo su - {app_name}"))?;

    // 5. Install nvm + node for the app user
    println!("→ installing nvm + node...");
    tmux_send(app_name, &pane,
        "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash")?;
    tmux_send(app_name, &pane, "export NVM_DIR=\"$HOME/.nvm\" && [ -s \"$NVM_DIR/nvm.sh\" ] && . \"$NVM_DIR/nvm.sh\"")?;
    tmux_send(app_name, &pane, "nvm install 22")?;

    // 6. Install uv for the app user
    println!("→ installing uv...");
    tmux_send(app_name, &pane, "curl -LsSf https://astral.sh/uv/install.sh | sh")?;
    tmux_send(app_name, &pane, "export PATH=\"$HOME/.local/bin:$PATH\"")?;

    // 7. Clone the repo
    if path_exists(&repo_dir) {
        println!("→ repo already cloned, skipping...");
    } else {
        wait_for_enter("press enter once the deploy key has been added...")?;
        println!("→ cloning {repo_url}...");
        tmux_send(app_name, &pane, &format!("cd {home}"))?;
        tmux_send(app_name, &pane, &format!(
            "GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new' git clone {repo_url}"
        ))?;
    }

    // 8. Generate gunicorn.py
    let gunicorn_path = format!("{home}/gunicorn.py");
    if path_exists(&gunicorn_path) {
        println!("→ gunicorn config already exists, skipping...");
    } else {
        println!("→ writing gunicorn config...");
        write_system_file(&gunicorn_path, &templates::gunicorn_config(app_name))?;
        run_cmd("sudo", &["chown", &format!("{app_name}:{app_name}"), &gunicorn_path])?;
    }

    // 9. Create PostgreSQL user + database
    let db_password = generate_password(32);
    if pg_role_exists(app_name) {
        println!("→ postgresql user already exists, skipping...");
    } else {
        println!("→ creating postgresql user...");
        run_cmd("sudo", &[
            "-u", "postgres", "psql", "-c",
            &format!("CREATE USER {app_name} WITH PASSWORD '{db_password}';"),
        ])?;
    }
    if pg_db_exists(app_name) {
        println!("→ postgresql database already exists, skipping...");
    } else {
        println!("→ creating postgresql database...");
        run_cmd("sudo", &[
            "-u", "postgres", "psql", "-c",
            &format!("CREATE DATABASE {app_name} OWNER {app_name};"),
        ])?;
    }

    // 10. Generate backend/.env
    let env_path = format!("{backend_dir}/.env");
    if path_exists(&env_path) {
        println!("→ .env already exists, skipping...");
    } else {
        println!("→ writing .env...");
        let secret_key = generate_secret(50);
        write_system_file(&env_path, &templates::dot_env(app_name, &domain, &secret_key, &db_password, &backend_dir))?;
        run_cmd("sudo", &["chown", &format!("{app_name}:{app_name}"), &env_path])?;
    }

    // 11. Backend: add gunicorn, sync deps, migrate
    println!("→ installing backend dependencies...");
    tmux_send(app_name, &pane, &format!("cd {backend_dir}"))?;
    tmux_send(app_name, &pane, "uv add gunicorn")?;
    tmux_send(app_name, &pane, "uv sync")?;
    println!("→ running migrations...");
    tmux_send(app_name, &pane, "uv run python manage.py migrate")?;

    // 12. Frontend: npm install + build
    println!("→ building frontend...");
    tmux_send(app_name, &pane, &format!("cd {frontend_dir}"))?;
    tmux_send(app_name, &pane, "npm install && npm run build")?;

    // 13. Collect static files
    println!("→ collecting static files...");
    tmux_send(app_name, &pane, &format!("cd {backend_dir}"))?;
    tmux_send(app_name, &pane, "uv run python manage.py collectstatic --noinput")?;

    // 14. Supervisor config
    println!("→ writing supervisor config...");
    let supervisor_path = format!("/etc/supervisor/conf.d/{app_name}.conf");
    write_system_file(&supervisor_path, &templates::supervisor_config(app_name, &repo_name))?;
    run_cmd("sudo", &["supervisorctl", "reread"])?;
    run_cmd("sudo", &["supervisorctl", "update"])?;

    // 15. Nginx config
    println!("→ writing nginx config...");
    let nginx_available = format!("/etc/nginx/sites-available/{app_name}");
    let nginx_enabled = format!("/etc/nginx/sites-enabled/{app_name}");
    write_system_file(&nginx_available, &templates::nginx_config(app_name, &repo_name, &domain))?;
    run_cmd("sudo", &["ln", "-sf", &nginx_available, &nginx_enabled])?;
    run_cmd("sudo", &["nginx", "-t"])?;
    run_cmd("sudo", &["systemctl", "reload", "nginx"])?;

    // 16. SSL via certbot
    println!("→ setting up ssl...");
    run_cmd("sudo", &["certbot", "--nginx", "-d", &domain, "--non-interactive", "--agree-tos", "--register-unsafely-without-email"])?;

    println!("\n✓ {app_name} provisioned at https://{domain}\n");

    // Attach to the tmux session
    run_cmd("tmux", &["attach-session", "-t", app_name])?;

    Ok(())
}

fn tmux_send(session: &str, pane: &str, cmd: &str) -> Result<(), String> {
    run_cmd("tmux", &["send-keys", "-t", pane, cmd, "Enter"])?;
    // Wait for the command to finish by checking for the prompt
    wait_for_pane(session, pane)
}

fn wait_for_pane(_session: &str, pane: &str) -> Result<(), String> {
    // Poll the pane until the shell is idle (last line ends with $ or #)
    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let content = run_cmd_output("tmux", &["capture-pane", "-t", pane, "-p"])?;
        if let Some(last_line) = content.lines().rev().find(|l| !l.is_empty()) {
            if last_line.ends_with('$') || last_line.ends_with('#') || last_line.ends_with('%') {
                return Ok(());
            }
        }
    }
}
