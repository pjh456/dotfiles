use std::process::Command;
use std::thread;

use anyhow::{Context, Result};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

pub fn install_exit_handler() -> Result<()> {
    let mut signals = Signals::new([SIGTERM, SIGINT]).context("install signal handler")?;
    thread::spawn(move || {
        if signals.forever().next().is_some() {
            std::process::exit(0);
        }
    });
    Ok(())
}

pub fn refresh_waybar() {
    let _ = Command::new("pkill").args(["-RTMIN+1", "waybar"]).status();
}

pub fn have_cmd(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .any(|candidate| candidate.is_file())
}
