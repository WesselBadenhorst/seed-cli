use crate::templates;
use crate::util::{
    generate_password, generate_secret, prompt, repo_name_from_url, run_cmd, run_cmd_as_user,
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
    println!("→ creating user {app_name}...");
    run_cmd("sudo", &["mkdir", "-p", "/webapps"])?;
    run_cmd("sudo", &[
        "useradd", "-m",
        "-d", &home,
        "-s", "/usr/bin/zsh",
        app_name,
    ])?;

    // 2. Create run/ and logs/ directories
    println!("→ creating directories...");
    run_cmd("sudo", &["-u", app_name, "mkdir", "-p", &format!("{home}/run")])?;
    run_cmd("sudo", &["-u", app_name, "mkdir", "-p", &format!("{home}/logs")])?;

    // 3. Clone the repo
    println!("→ cloning {repo_url}...");
    run_cmd("sudo", &["-u", app_name, "git", "clone", repo_url, &repo_dir])?;

    // 4. Generate gunicorn.py
    println!("→ writing gunicorn config...");
    let gunicorn_path = format!("{home}/gunicorn.py");
    write_system_file(&gunicorn_path, &templates::gunicorn_config(app_name))?;
    run_cmd("sudo", &["chown", &format!("{app_name}:{app_name}"), &gunicorn_path])?;

    // 5. Create PostgreSQL user + database
    println!("→ setting up postgresql...");
    let db_password = generate_password(32);
    run_cmd("sudo", &[
        "-u", "postgres", "psql", "-c",
        &format!("CREATE USER {app_name} WITH PASSWORD '{db_password}';"),
    ])?;
    run_cmd("sudo", &[
        "-u", "postgres", "psql", "-c",
        &format!("CREATE DATABASE {app_name} OWNER {app_name};"),
    ])?;

    // 6. Generate backend/.env
    println!("→ writing .env...");
    let secret_key = generate_secret(50);
    let env_path = format!("{backend_dir}/.env");
    write_system_file(&env_path, &templates::dot_env(app_name, &domain, &secret_key, &db_password))?;
    run_cmd("sudo", &["chown", &format!("{app_name}:{app_name}"), &env_path])?;

    // 7. Install backend dependencies + migrate + collectstatic
    println!("→ installing backend dependencies...");
    run_cmd_as_user(app_name, &format!("cd {backend_dir} && uv sync"))?;
    println!("→ running migrations...");
    run_cmd_as_user(app_name, &format!("cd {backend_dir} && uv run python manage.py migrate"))?;

    // 8. Install frontend + build
    println!("→ building frontend...");
    run_cmd_as_user(app_name, &format!("cd {frontend_dir} && npm install && npm run build"))?;

    // 9. Collect static files
    println!("→ collecting static files...");
    run_cmd_as_user(app_name, &format!("cd {backend_dir} && uv run python manage.py collectstatic --noinput"))?;

    // 10. Supervisor config
    println!("→ writing supervisor config...");
    let supervisor_path = format!("/etc/supervisor/conf.d/{app_name}.conf");
    write_system_file(&supervisor_path, &templates::supervisor_config(app_name, &repo_name))?;
    run_cmd("sudo", &["supervisorctl", "reread"])?;
    run_cmd("sudo", &["supervisorctl", "update"])?;

    // 11. Nginx config
    println!("→ writing nginx config...");
    let nginx_available = format!("/etc/nginx/sites-available/{app_name}");
    let nginx_enabled = format!("/etc/nginx/sites-enabled/{app_name}");
    write_system_file(&nginx_available, &templates::nginx_config(app_name, &repo_name, &domain))?;
    run_cmd("sudo", &["ln", "-sf", &nginx_available, &nginx_enabled])?;
    run_cmd("sudo", &["nginx", "-t"])?;
    run_cmd("sudo", &["systemctl", "reload", "nginx"])?;

    // 12. SSL via certbot
    println!("→ setting up ssl...");
    run_cmd("sudo", &["certbot", "--nginx", "-d", &domain, "--non-interactive", "--agree-tos", "--register-unsafely-without-email"])?;

    // 13. Start tmux session
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
