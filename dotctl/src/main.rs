mod bluez;
mod cli;
mod cmd;
mod osd;
mod power;
mod reload;
mod sysfs;
mod wallpaper;
mod waybar;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    if let Err(err) = cmd::run(cli) {
        eprintln!("dotctl: {err:#}");
        std::process::exit(1);
    }
}
