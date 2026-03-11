use crate::util::{path_exists, pg_db_exists, pg_role_exists, prompt, run_cmd, user_exists};

pub fn remove(app_name: &str) -> Result<(), String> {
    let home = format!("/webapps/{app_name}");

    println!("this will remove everything for '{app_name}':");
    println!("  • supervisor program");
    println!("  • nginx config + ssl certificate");
    println!("  • postgresql database + user");
    println!("  • system user + {home}\n");

    let answer = prompt("type the app name to confirm")?;
    if answer != app_name {
        return Err("confirmation did not match — aborting".to_string());
    }

    // 1. Stop and remove supervisor program
    let supervisor_conf = format!("/etc/supervisor/conf.d/{app_name}.conf");
    if path_exists(&supervisor_conf) {
        println!("→ stopping supervisor program...");
        let _ = run_cmd("sudo", &["supervisorctl", "stop", app_name]);
        run_cmd("sudo", &["rm", "-f", &supervisor_conf])?;
        run_cmd("sudo", &["supervisorctl", "reread"])?;
        run_cmd("sudo", &["supervisorctl", "update"])?;
    } else {
        println!("→ no supervisor config found, skipping...");
    }

    // 2. Remove nginx config
    let nginx_available = format!("/etc/nginx/sites-available/{app_name}");
    let nginx_enabled = format!("/etc/nginx/sites-enabled/{app_name}");
    if path_exists(&nginx_available) || path_exists(&nginx_enabled) {
        println!("→ removing nginx config...");
        run_cmd("sudo", &["rm", "-f", &nginx_available])?;
        run_cmd("sudo", &["rm", "-f", &nginx_enabled])?;
        run_cmd("sudo", &["nginx", "-t"])?;
        run_cmd("sudo", &["systemctl", "reload", "nginx"])?;
    } else {
        println!("→ no nginx config found, skipping...");
    }

    // 3. Remove certbot certificate
    println!("→ removing ssl certificate...");
    let _ = run_cmd("sudo", &[
        "certbot", "delete", "--cert-name", app_name, "--non-interactive",
    ]);

    // 4. Drop PostgreSQL database
    if pg_db_exists(app_name) {
        println!("→ dropping postgresql database...");
        run_cmd("sudo", &[
            "-u", "postgres", "psql", "-c",
            &format!("DROP DATABASE {app_name};"),
        ])?;
    } else {
        println!("→ no postgresql database found, skipping...");
    }

    // 5. Drop PostgreSQL user
    if pg_role_exists(app_name) {
        println!("→ dropping postgresql user...");
        run_cmd("sudo", &[
            "-u", "postgres", "psql", "-c",
            &format!("DROP USER {app_name};"),
        ])?;
    } else {
        println!("→ no postgresql user found, skipping...");
    }

    // 6. End tmux session
    let _ = run_cmd("tmux", &["kill-session", "-t", app_name]);

    // 7. Remove system user + home directory
    if user_exists(app_name) {
        println!("→ removing user {app_name} and {home}...");
        run_cmd("sudo", &["userdel", "-r", app_name])?;
    } else {
        println!("→ user {app_name} does not exist, skipping...");
    }

    // Clean up /webapps/<app> if userdel didn't remove it
    if path_exists(&home) {
        println!("→ cleaning up leftover files...");
        run_cmd("sudo", &["rm", "-rf", &home])?;
    }

    println!("\n✓ {app_name} fully removed\n");
    Ok(())
}
