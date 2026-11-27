mod bluez;
mod cli;
mod cmd;
mod daemon;
mod guard;
mod menu;
mod mpv;
mod notify;
mod osd;
mod power;
mod power_watch;
mod ppd;
mod reload;
mod rofi;
mod sysfs;
mod wallpaper;
mod waybar;
mod weather;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    if let Err(err) = cmd::run(cli) {
        eprintln!("dotctl: {err:#}");
        std::process::exit(1);
    }
}
