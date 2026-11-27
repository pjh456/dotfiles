use std::env;
use std::path::Path;

use anyhow::Result;

use crate::sysfs;

pub fn run() -> Result<()> {
    println!("dotctl {}", env!("CARGO_PKG_VERSION"));
    println!(
        "os        {}",
        os_pretty_name().unwrap_or_else(|| "unknown".into())
    );

    let session = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    let wayland = env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "-".into());
    println!("session   {session} (WAYLAND_DISPLAY={wayland})");

    match sysfs::cpu_temp() {
        Some(temp) => println!("cpu temp  {temp}°C"),
        None => println!("cpu temp  unavailable (no coretemp/k10temp/acpitz hwmon)"),
    }

    let runtime = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".into());
    println!(
        "user bus  {}",
        yes_no(Path::new(&runtime).join("bus").exists())
    );
    println!(
        "sys bus   {}",
        yes_no(Path::new("/run/dbus/system_bus_socket").exists())
    );

    let hypr_socket = env::var("HYPRLAND_INSTANCE_SIGNATURE")
        .ok()
        .map(|sig| format!("{runtime}/hypr/{sig}/.socket.sock"));
    println!(
        "hyprland  {}",
        hypr_socket
            .as_deref()
            .map(|p| yes_no(Path::new(p).exists()))
            .unwrap_or("no HYPRLAND_INSTANCE_SIGNATURE")
    );

    println!();
    println!("commands:");
    for cmd in [
        "rofi",
        "wl-copy",
        "cliphist",
        "notify-send",
        "wpctl",
        "brightnessctl",
        "hyprctl",
        "gdb",
        "wtype",
        "bluetoothctl",
    ] {
        println!("  {:<16} {}", cmd, yes_no(have_cmd(cmd)));
    }

    Ok(())
}

fn os_pretty_name() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    text.lines().find_map(|line| {
        line.strip_prefix("PRETTY_NAME=")
            .map(|value| value.trim_matches('"').to_owned())
    })
}

fn have_cmd(cmd: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| {
        let candidate = dir.join(cmd);
        candidate.is_file()
    })
}

fn yes_no(value: bool) -> &'static str {
    if value { "ok" } else { "missing" }
}
