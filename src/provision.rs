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

    // 3. Generate SSH key for the user and wait for deploy key setup
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

    // 4. Clone the repo as the app user
    if path_exists(&repo_dir) {
        println!("→ repo already cloned, skipping...");
    } else {
        wait_for_enter("press enter once the deploy key has been added...")?;
        println!("→ cloning {repo_url}...");
        run_cmd_as_user(app_name, &format!(
            "GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new' git clone {repo_url} {repo_dir}"
        ))?;
    }

    // 5. Generate gunicorn.py
    let gunicorn_path = format!("{home}/gunicorn.py");
    if path_exists(&gunicorn_path) {
        println!("→ gunicorn config already exists, skipping...");
    } else {
        println!("→ writing gunicorn config...");
        write_system_file(&gunicorn_path, &templates::gunicorn_config(app_name))?;
        run_cmd("sudo", &["chown", &format!("{app_name}:{app_name}"), &gunicorn_path])?;
    }

    // 6. Create PostgreSQL user + database
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

    // 7. Generate backend/.env
    let env_path = format!("{backend_dir}/.env");
    if path_exists(&env_path) {
        println!("→ .env already exists, skipping...");
    } else {
        println!("→ writing .env...");
        let secret_key = generate_secret(50);
        write_system_file(&env_path, &templates::dot_env(app_name, &domain, &secret_key, &db_password))?;
        run_cmd("sudo", &["chown", &format!("{app_name}:{app_name}"), &env_path])?;
    }

    // 8. Install uv for the app user
    println!("→ installing uv...");
    run_cmd_as_user(app_name, "curl -LsSf https://astral.sh/uv/install.sh | sh")?;

    // 9. Install backend dependencies + migrate
    println!("→ adding gunicorn...");
    run_cmd_as_user(app_name, &format!("cd {backend_dir} && ~/.local/bin/uv add gunicorn"))?;
    println!("→ installing backend dependencies...");
    run_cmd_as_user(app_name, &format!("cd {backend_dir} && ~/.local/bin/uv sync"))?;
    println!("→ running migrations...");
    run_cmd_as_user(app_name, &format!("cd {backend_dir} && ~/.local/bin/uv run python manage.py migrate"))?;

    // 9. Install frontend + build
    println!("→ building frontend...");
    run_cmd_as_user(app_name, &format!("cd {frontend_dir} && npm install && npm run build"))?;

    // 11. Collect static files
    println!("→ collecting static files...");
    run_cmd_as_user(app_name, &format!("cd {backend_dir} && ~/.local/bin/uv run python manage.py collectstatic --noinput"))?;

    // 11. Supervisor config
    println!("→ writing supervisor config...");
    let supervisor_path = format!("/etc/supervisor/conf.d/{app_name}.conf");
    write_system_file(&supervisor_path, &templates::supervisor_config(app_name, &repo_name))?;
    run_cmd("sudo", &["supervisorctl", "reread"])?;
    run_cmd("sudo", &["supervisorctl", "update"])?;

    // 12. Nginx config
    println!("→ writing nginx config...");
    let nginx_available = format!("/etc/nginx/sites-available/{app_name}");
    let nginx_enabled = format!("/etc/nginx/sites-enabled/{app_name}");
    write_system_file(&nginx_available, &templates::nginx_config(app_name, &repo_name, &domain))?;
    run_cmd("sudo", &["ln", "-sf", &nginx_available, &nginx_enabled])?;
    run_cmd("sudo", &["nginx", "-t"])?;
    run_cmd("sudo", &["systemctl", "reload", "nginx"])?;

    // 13. SSL via certbot
    println!("→ setting up ssl...");
    run_cmd("sudo", &["certbot", "--nginx", "-d", &domain, "--non-interactive", "--agree-tos", "--register-unsafely-without-email"])?;

    // 14. Start tmux session
    println!("\n✓ {app_name} provisioned at https://{domain}\n");

    run_cmd("tmux", &["new-session", "-d", "-s", app_name])?;
    run_cmd("tmux", &["split-window", "-h", "-t", app_name])?;
    run_cmd("tmux", &[
        "send-keys", "-t", &format!("{app_name}:0.0"),
        &format!("sudo su - {app_name}"), "Enter",
    ])?;
    run_cmd("tmux", &["attach-session", "-t", app_name])?;

    Ok(())
}
