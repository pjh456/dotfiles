mod doctor;
mod status;

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::{
    BtAction, Cli, Command, MenuKind, OsdTarget, ReloadTarget, WallpaperAction, WatchKind,
};
use crate::{
    bluez, guard, install, menu, mpv, osd, power, power_watch, reload, rofi, wallpaper, weather,
};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Status { kind } => status::run(kind),
        Command::Bt { action } => match action {
            BtAction::Status => {
                status::bt().print();
                Ok(())
            }
            BtAction::Toggle => bluez::toggle(),
        },
        Command::Osd { target } => match target {
            OsdTarget::Volume { action } => osd::volume(action),
            OsdTarget::Brightness { action } => osd::brightness(action),
        },
        Command::Wallpaper { action } => match action {
            WallpaperAction::Set { path } => wallpaper::set(&path),
            WallpaperAction::Unset => wallpaper::unset(),
        },
        Command::OnBattery => {
            std::process::exit(if power::ac_online() { 1 } else { 0 });
        }
        Command::Reload { target } => match target {
            ReloadTarget::Waybar => reload::waybar(),
        },
        Command::Rofi { args } => rofi::toggle(&args),
        Command::Menu { kind } => match kind {
            MenuKind::Clipboard => menu::clipboard(),
            MenuKind::Emoji { paste } => menu::emoji(paste),
            MenuKind::Power => menu::power(),
            MenuKind::Launcher => menu::launcher(),
        },
        Command::Weather {
            forecast,
            refresh,
            retry,
        } => {
            if refresh {
                if retry {
                    weather::refresh_with_retries()
                } else {
                    weather::refresh()
                }
            } else if forecast {
                weather::forecast()
            } else {
                weather::status()
            }
        }
        Command::Watch { kind } => match kind {
            WatchKind::Mpv { once } => mpv::watch(once),
            WatchKind::Power { once } => power_watch::watch(once),
        },
        Command::Guard { once, state_dir } => guard::run(once, state_dir),
        Command::Mpv { profile } => mpv::switch(profile.as_deref()),
        Command::Install { repo_root, home } => {
            install::install(&repo_root, &home.unwrap_or_else(default_home))
        }
        Command::Restore { dir, home } => install::restore(&home.unwrap_or_else(default_home), dir),
        Command::Doctor => doctor::run(),
    }
}

fn default_home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
        .unwrap_or_else(|err| {
            eprintln!("dotctl: {err:#}");
            std::process::exit(1);
        })
}
