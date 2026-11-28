use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::StreamExt;

use crate::{daemon, ppd};

pub fn switch(profile: Option<&str>) -> Result<()> {
    let raw = match profile {
        Some(profile) => profile.to_owned(),
        None => current_profile(),
    };
    let target = target(&raw);

    let home = env::var_os("HOME").context("HOME is not set")?;
    let dir = PathBuf::from(home).join(".config").join("mpv");
    let link = dir.join("mpv.conf");
    let want = dir.join(format!("mpv.conf.{target}"));

    if fs::read_link(&link).ok().as_deref() == Some(want.as_path()) {
        return Ok(());
    }

    if link.symlink_metadata().is_ok() {
        fs::remove_file(&link).with_context(|| format!("remove {}", link.display()))?;
    }
    symlink(&want, &link).with_context(|| format!("link {}", link.display()))?;
    println!("[mpv-profile] switched to {target}");
    Ok(())
}

pub fn watch(once: bool) -> Result<()> {
    daemon::install_exit_handler()?;

    if once {
        let connection = zbus::block_on(ppd::connect())?;
        let initial =
            zbus::block_on(ppd::active_profile(&connection)).unwrap_or_else(|_| current_profile());
        switch(Some(&initial))?;
        return Ok(());
    }

    // PPD or the system bus can restart independently of this service. A
    // closed PropertiesChanged stream is not fatal: reconnect and resubscribe.
    loop {
        if let Err(err) = watch_session() {
            eprintln!("dotctl: mpv watch: {err:#}");
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn watch_session() -> Result<()> {
    let connection = zbus::block_on(ppd::connect())?;
    let initial =
        zbus::block_on(ppd::active_profile(&connection)).unwrap_or_else(|_| current_profile());
    if let Err(err) = switch(Some(&initial)) {
        eprintln!("dotctl: mpv watch: initial switch: {err:#}");
    }

    zbus::block_on(subscribe(&connection))
}

async fn subscribe(connection: &zbus::Connection) -> Result<()> {
    let proxy = zbus::fdo::PropertiesProxy::builder(connection)
        .destination(ppd::INTERFACE)
        .path(ppd::PATH)
        .build()
        .await?;
    let mut stream = proxy.receive_properties_changed().await?;

    while let Some(message) = stream.next().await {
        let args = match message.args() {
            Ok(args) => args,
            Err(err) => {
                eprintln!("dotctl: mpv watch: invalid PropertiesChanged: {err}");
                continue;
            }
        };
        if args.interface_name.as_str() != ppd::INTERFACE {
            continue;
        }
        let Some(value) = args.changed_properties.get("ActiveProfile") else {
            continue;
        };
        let Ok(profile) = value.downcast_ref::<&str>() else {
            continue;
        };

        if let Err(err) = switch(Some(profile)) {
            eprintln!("dotctl: mpv watch: {err:#}");
        }
        daemon::refresh_waybar();
    }

    Ok(())
}

fn current_profile() -> String {
    ppd::active_profile_now()
        .unwrap_or_else(|_| tlp_profile().unwrap_or_else(|| "balanced".to_owned()))
}

fn tlp_profile() -> Option<String> {
    let output = Command::new("tlp-stat").args(["-s"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    parse_tlp_profile(&String::from_utf8_lossy(&output.stdout))
}

fn parse_tlp_profile(text: &str) -> Option<String> {
    for line in text.lines() {
        if !line.contains("TLP profile") {
            continue;
        }
        let (_, value) = line.split_once('=')?;
        return Some(value.trim().to_owned());
    }
    None
}

fn target(profile: &str) -> &'static str {
    let profile = profile.split('/').next().unwrap_or(profile);
    match profile {
        "PRF" | "performance" => "PRF",
        "SAV" | "save" | "powersave" | "power-saver" => "SAV",
        _ => "BAL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tlp_stat_profile() {
        let text = "State = enabled\nTLP profile = performance/BAT\n";
        assert_eq!(parse_tlp_profile(text).as_deref(), Some("performance/BAT"));
    }

    #[test]
    fn maps_known_profile_names() {
        assert_eq!(target("performance"), "PRF");
        assert_eq!(target("PRF"), "PRF");
        assert_eq!(target("balanced/BAT"), "BAL");
        assert_eq!(target("power-saver"), "SAV");
        assert_eq!(target("SAV"), "SAV");
        assert_eq!(target("unknown"), "BAL");
    }
}
