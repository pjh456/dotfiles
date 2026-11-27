mod bluez;
mod cli;
mod cmd;
mod sysfs;
mod waybar;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

    if let Err(err) = cmd::run(cli) {
        eprintln!("dotctl: {err:#}");
        std::process::exit(1);
    }
}
