# seed 🌱

A personal provisioning tool for spinning up Django + Gunicorn web apps on Ubuntu servers running nginx, supervisor, and postgres.

It is opinionated, fast, and designed for a solo developer who wants to go from zero to running app without repeating the same setup steps every time.

## Install

Requires [Rust](https://rustup.rs) and git.

```bash
curl -fsSL https://raw.githubusercontent.com/WesselBadenhorst/seed-cli/main/install.sh | bash
```

## Upgrade

```bash
seed upgrade
```

## Status

Evolving alongside real deployments. Breaking changes happen. That's fine.
