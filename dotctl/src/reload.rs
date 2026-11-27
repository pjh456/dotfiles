use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

/// Restart Waybar. A SIGUSR2 "soft reload" leaves stale GTK widgets behind in
/// waybar 0.16, so a real restart is the reliable way to pick up config edits.
pub fn waybar() -> Result<()> {
    if Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "waybar.service"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    {
        let status = Command::new("systemctl")
            .args(["--user", "restart", "waybar.service"])
            .status()
            .context("spawn systemctl --user restart waybar.service")?;
        if !status.success() {
            bail!("systemctl --user restart waybar.service failed with {status}");
        }
        return Ok(());
    }

    let _ = Command::new("pkill").args(["-x", "waybar"]).status();
    for _ in 0..50 {
        let still_running = Command::new("pgrep")
            .args(["-x", "waybar"])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !still_running {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }

    Command::new("waybar").spawn().context("start waybar")?;
    Ok(())
}
