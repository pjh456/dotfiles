use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "dotctl",
    version,
    about = "Desktop control helpers for my Hyprland dotfiles",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print a Waybar-compatible JSON status line
    Status {
        #[command(subcommand)]
        kind: StatusKind,
    },

    /// Check runtime dependencies and report what is missing
    Doctor,
}

#[derive(Debug, Subcommand)]
pub enum StatusKind {
    /// CPU package temperature
    Temp,
}
