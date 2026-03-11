pub fn gunicorn_config(app_name: &str) -> String {
    format!(
        r#"import multiprocessing

# Socket
bind = "unix:/webapps/{app_name}/run/gunicorn.sock"

# Workers
workers = multiprocessing.cpu_count() * 2 + 1
worker_class = "sync"
worker_connections = 1000
timeout = 30
keepalive = 2

# Logging
accesslog = "/webapps/{app_name}/logs/gunicorn-access.log"
errorlog = "/webapps/{app_name}/logs/gunicorn-error.log"
loglevel = "info"

# Process naming
proc_name = "{app_name}"

# Server mechanics
daemon = False
pidfile = "/webapps/{app_name}/run/gunicorn.pid"
umask = 0o007
tmp_upload_dir = None
"#
    )
}

pub fn supervisor_config(app_name: &str, repo_name: &str) -> String {
    let home = format!("/webapps/{app_name}");
    let backend_dir = format!("{home}/{repo_name}/backend");
    format!(
        r#"[program:{app_name}]
directory={backend_dir}
command={backend_dir}/.venv/bin/gunicorn app.wsgi:application -c {home}/gunicorn.py
user={app_name}
autostart=true
autorestart=true
stderr_logfile={home}/logs/supervisor-err.log
stdout_logfile={home}/logs/supervisor-out.log
"#
    )
}

pub fn nginx_config(app_name: &str, repo_name: &str, domain: &str) -> String {
    let home = format!("/webapps/{app_name}");
    let static_dir = format!("{home}/{repo_name}/backend/static");
    format!(
        r#"upstream {app_name}_app {{
    server unix:{home}/run/gunicorn.sock fail_timeout=0;
}}

server {{
    listen 80;
    server_name {domain};

    client_max_body_size 4M;

    root {static_dir};
    index index.html;

    location /static/ {{
        alias {static_dir}/;
        expires 30d;
        add_header Cache-Control "public, immutable";
    }}

    location / {{
        try_files $uri /index.html;
    }}

    location /api/ {{
        proxy_pass http://{app_name}_app;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_redirect off;
    }}

    location /admin/ {{
        proxy_pass http://{app_name}_app;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_redirect off;
    }}
}}
"#
    )
}

pub fn dot_env(app_name: &str, domain: &str, secret_key: &str, db_password: &str) -> String {
    format!(
        r#"# ==========================
# Production Environment
# ==========================

# Core
DEBUG=False
SECRET_KEY={secret_key}
ALLOWED_HOSTS={domain}

# Django
DJANGO_SETTINGS_MODULE=app.settings.prod

# Database
DATABASE_URL=postgres://{app_name}:{db_password}@localhost/{app_name}

# Auth
SITE_ID=1
"#
    )
}
