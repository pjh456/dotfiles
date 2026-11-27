use std::path::PathBuf;

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

    /// Bluetooth status and on-demand toggle
    Bt {
        #[command(subcommand)]
        action: BtAction,
    },

    /// Volume and brightness with a swaync OSD
    Osd {
        #[command(subcommand)]
        target: OsdTarget,
    },

    /// Wallpaper helpers used by hyprpaper and hyprlock
    Wallpaper {
        #[command(subcommand)]
        action: WallpaperAction,
    },

    /// Exit 0 when on battery, 1 when AC is connected
    OnBattery,

    /// Restart a desktop component
    Reload {
        #[command(subcommand)]
        target: ReloadTarget,
    },

    /// Toggle a rofi menu (closes it when already open)
    Rofi {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Scripted rofi menus
    Menu {
        #[command(subcommand)]
        kind: MenuKind,
    },

    /// Waybar weather status, or the 7-day rofi forecast
    Weather {
        #[arg(long)]
        forecast: bool,
        #[arg(long, hide = true)]
        refresh: bool,
    },

    /// Long-running watchers (mpv profile, AC power profile)
    Watch {
        #[command(subcommand)]
        kind: WatchKind,
    },

    /// Mirror the Hyprland log and capture a stack trace when it hangs
    Guard {
        #[arg(long)]
        once: bool,
        #[arg(long)]
        state_dir: Option<PathBuf>,
    },

    /// Force-sync the mpv profile now (no argument: read the current PPD profile)
    Mpv { profile: Option<String> },

    /// Check runtime dependencies and report what is missing
    Doctor,
}

#[derive(Debug, Subcommand)]
pub enum StatusKind {
    /// CPU package temperature
    Temp,
    /// On-demand Bluetooth state
    Bt,
}

#[derive(Debug, Subcommand)]
pub enum BtAction {
    Status,
    Toggle,
}

#[derive(Debug, Subcommand)]
pub enum OsdTarget {
    Volume {
        #[command(subcommand)]
        action: VolumeAction,
    },
    Brightness {
        #[command(subcommand)]
        action: BrightnessAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum VolumeAction {
    Up,
    Down,
    Mute,
}

#[derive(Debug, Subcommand)]
pub enum BrightnessAction {
    Up,
    Down,
}

#[derive(Debug, Subcommand)]
pub enum WallpaperAction {
    Set { path: PathBuf },
    Unset,
}

#[derive(Debug, Subcommand)]
pub enum ReloadTarget {
    Waybar,
}

#[derive(Debug, Subcommand)]
pub enum WatchKind {
    Mpv {
        #[arg(long)]
        once: bool,
    },
    Power {
        #[arg(long)]
        once: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum MenuKind {
    Clipboard,
    Emoji {
        #[arg(long)]
        paste: bool,
    },
    Power,
    Launcher,
}
