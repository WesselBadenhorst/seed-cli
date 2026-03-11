use crate::util::command_exists;

struct Dependency {
    name: &'static str,
    binary: &'static str,
    install_hint: &'static str,
}

const DEPENDENCIES: &[Dependency] = &[
    Dependency {
        name: "tmux",
        binary: "tmux",
        install_hint: "sudo apt install tmux",
    },
    Dependency {
        name: "neovim",
        binary: "nvim",
        install_hint: "sudo apt install neovim",
    },
    Dependency {
        name: "nginx",
        binary: "nginx",
        install_hint: "sudo apt install nginx",
    },
    Dependency {
        name: "supervisor",
        binary: "supervisord",
        install_hint: "sudo apt install supervisor",
    },
    Dependency {
        name: "postgresql",
        binary: "psql",
        install_hint: "sudo apt install postgresql",
    },
];

pub fn preflight_check() -> Result<(), Vec<(&'static str, &'static str)>> {
    let missing: Vec<_> = DEPENDENCIES
        .iter()
        .filter(|dep| !command_exists(dep.binary))
        .map(|dep| (dep.name, dep.install_hint))
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}
