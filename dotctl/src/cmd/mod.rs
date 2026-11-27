mod doctor;
mod status;

use anyhow::Result;

use crate::cli::{BtAction, Cli, Command, OsdTarget, ReloadTarget, WallpaperAction};
use crate::{bluez, osd, power, reload, wallpaper};

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
        Command::Doctor => doctor::run(),
    }
}
