# seedapp 🌱

A personal provisioning tool for spinning up Django + Gunicorn web apps on Ubuntu servers running nginx, supervisor, and postgres.

It is opinionated, fast, and designed for a solo developer who wants to go from zero to running app without repeating the same setup steps every time.

## What it does

One command creates a fully provisioned app environment:

```bash
seedapp new <app-name> <repo-url>
```

### What gets set up

**System user**
- Creates a dedicated user `<app-name>` with primary group `www-data`
- Home directory: `/webapps/<app-name>`
- Shell: zsh

**App directory**
```
/webapps/<app-name>/
├── <repo>/     ← git clone of your repo
├── run/        ← gunicorn socket
├── logs/       ← app logs
└── .venv/      ← Python virtual environment
```

**Shell environment**
- zsh + oh-my-zsh
- Lambda theme
- zsh-autosuggestions

**Neovim**
- lazy.nvim
- tokyonight-night colorscheme
- mason + nvim-lspconfig
- nvim-treesitter
- harpoon2
- `<leader>` = Space
- `<leader>pv` → `:Ex`

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/WesselBadenhorst/seed-cli/main/install.sh | bash
```

## Usage

```bash
# Provision a new web app
seedapp new myapp git@github.com:you/myapp.git

# Check version
seedapp --version
```

## Assumptions

- Ubuntu server with nginx, supervisor, and postgres already running
- Django app served with gunicorn
- Run as root or with sudo
- Apps live under `/webapps/`

## Philosophy

seedapp is intentionally:
- ❌ Not generic
- ❌ Not cross-platform
- ❌ Not configurable beyond what's needed

It **is**:
- fast
- repeatable
- exactly what I need

## Status

Evolving alongside real deployments. Breaking changes happen. That's fine.
