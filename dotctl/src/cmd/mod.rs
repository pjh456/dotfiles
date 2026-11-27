mod doctor;
mod status;

use anyhow::Result;

use crate::cli::{Cli, Command};

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Status { kind } => status::run(kind),
        Command::Doctor => doctor::run(),
    }
}
