use std::thread;
use std::time::Duration;

use anyhow::Result;

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

    // sysfs does not reliably deliver inotify events for power_supply changes
    // on this hardware (TLP ended up being the only actor). Reading three tiny
    // files once a second is cheaper and far more reliable than a missed
    // transition; this is still well below the old busctl/tlp-stat polling.
    loop {
        thread::sleep(Duration::from_secs(1));

        let now_on_ac = power::ac_online();
        if now_on_ac == on_ac {
            continue;
        }
        // Transition-only on purpose: a manual profile change made after a
        // plug/unplug must stick until the next transition.
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
