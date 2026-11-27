use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use inotify::{Inotify, WatchMask};

use crate::{daemon, power, ppd};

pub fn watch(once: bool) -> Result<()> {
    daemon::install_exit_handler()?;

    let connection = zbus::block_on(ppd::connect())?;
    let mut on_ac = power::ac_online();
    set_for_ac(&connection, on_ac);

    if once {
        println!(
            "{}",
            if on_ac {
                "AC online (performance)"
            } else {
                "on battery (power-saver)"
            }
        );
        return Ok(());
    }

    let mut inotify = Inotify::init().context("init inotify")?;
    let mut watched = 0;
    for path in online_files(Path::new("/sys/class/power_supply")) {
        if inotify
            .watches()
            .add(&path, WatchMask::MODIFY)
            .with_context(|| format!("watch {}", path.display()))
            .is_ok()
        {
            watched += 1;
        }
    }
    if watched == 0 {
        bail!("no /sys/class/power_supply/*/online files to watch");
    }

    let mut buffer = [0u8; 4096];
    loop {
        inotify
            .read_events_blocking(&mut buffer)
            .context("read inotify events")?;

        let now_on_ac = power::ac_online();
        if now_on_ac == on_ac {
            continue;
        }
        on_ac = now_on_ac;
        set_for_ac(&connection, on_ac);
    }
}

fn set_for_ac(connection: &zbus::Connection, on_ac: bool) {
    let wanted = if on_ac { "performance" } else { "power-saver" };

    let current = match zbus::block_on(ppd::active_profile(connection)) {
        Ok(profile) => profile,
        Err(err) => {
            eprintln!("dotctl: power watch: read profile: {err:#}");
            return;
        }
    };
    if current == wanted {
        return;
    }

    match zbus::block_on(ppd::set_profile(connection, wanted)) {
        Ok(()) => daemon::refresh_waybar(),
        Err(err) => eprintln!("dotctl: power watch: set {wanted}: {err:#}"),
    }
}

fn online_files(root: &Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path().join("online"))
        .filter(|path| path.is_file())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dotctl-watch-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_only_existing_online_files() {
        let root = scratch();
        fs::create_dir_all(root.join("AC")).unwrap();
        fs::write(root.join("AC/online"), "1").unwrap();
        fs::create_dir_all(root.join("BAT0")).unwrap();
        fs::write(root.join("BAT0/type"), "Battery").unwrap();

        let files = online_files(&root);
        assert_eq!(files, vec![root.join("AC/online")]);
        fs::remove_dir_all(root).unwrap();
    }
}
