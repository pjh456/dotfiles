use std::fs;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use notify_rust::{Hint, Notification, Timeout};

use crate::cli::{BrightnessAction, VolumeAction};

const STEP: &str = "1%";
const SINK: &str = "@DEFAULT_AUDIO_SINK@";

pub fn volume(action: VolumeAction) -> Result<()> {
    match action {
        VolumeAction::Up | VolumeAction::Down => {
            run(Command::new("wpctl").args(["set-mute", SINK, "0"]))?;
            let step = match action {
                VolumeAction::Up => format!("{STEP}+"),
                VolumeAction::Down => format!("{STEP}-"),
                VolumeAction::Mute => unreachable!(),
            };
            run(Command::new("wpctl").args(["set-volume", SINK, &step]))?;
        }
        VolumeAction::Mute => {
            run(Command::new("wpctl").args(["set-mute", SINK, "toggle"]))?;
        }
    }

    let state = output(Command::new("wpctl").args(["get-volume", SINK]))?;
    let muted = state.contains("MUTED");
    let pct = state
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| (value * 100.0) as i32)
        .unwrap_or(0);

    let (icon, text) = if muted {
        ("audio-volume-muted", "Muted".to_owned())
    } else if pct >= 66 {
        ("audio-volume-high", format!("{pct}%"))
    } else if pct >= 33 {
        ("audio-volume-medium", format!("{pct}%"))
    } else {
        ("audio-volume-low", format!("{pct}%"))
    };

    notify(pct, icon, "Volume", &text);
    Ok(())
}

pub fn brightness(action: BrightnessAction) -> Result<()> {
    let step = match action {
        BrightnessAction::Up => format!("{STEP}+"),
        BrightnessAction::Down => format!("{STEP}-"),
    };
    if let Err(err) = run(Command::new("brightnessctl")
        .args(["set", &step])
        .stdout(Stdio::null()))
    {
        // A missing udev rule or the video group is common; still show the
        // current value instead of leaving the keypress silent.
        eprintln!("dotctl: could not change brightness: {err:#}");
    }

    let pct = backlight_percent()
        .or_else(brightnessctl_percent)
        .unwrap_or(0);
    let icon = if pct >= 66 {
        "display-brightness-high"
    } else if pct >= 33 {
        "display-brightness-medium"
    } else {
        "display-brightness-low"
    };

    notify(pct, icon, "Brightness", &format!("{pct}%"));
    Ok(())
}

fn backlight_percent() -> Option<i32> {
    for entry in fs::read_dir("/sys/class/backlight").ok()?.flatten() {
        let dir = entry.path();
        let current = fs::read_to_string(dir.join("brightness")).ok()?;
        let max = fs::read_to_string(dir.join("max_brightness")).ok()?;
        let current: i64 = current.trim().parse().ok()?;
        let max: i64 = max.trim().parse().ok()?;
        if max > 0 {
            return Some((current * 100 / max) as i32);
        }
    }
    None
}

fn brightnessctl_percent() -> Option<i32> {
    let current: i64 = output(Command::new("brightnessctl").arg("get"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let max: i64 = output(Command::new("brightnessctl").arg("max"))
        .ok()?
        .trim()
        .parse()
        .ok()?;
    (max > 0).then(|| (current * 100 / max) as i32)
}

fn notify(value: i32, icon: &str, summary: &str, body: &str) {
    let result = Notification::new()
        .appname("osd")
        .icon(icon)
        .summary(summary)
        .body(body)
        .hint(Hint::Custom(
            "x-canonical-private-synchronous".to_owned(),
            "osd".to_owned(),
        ))
        .hint(Hint::CustomInt("int:value".to_owned(), value))
        .timeout(Timeout::Milliseconds(1200))
        .show();

    if let Err(err) = result {
        eprintln!("dotctl: could not show the OSD notification: {err}");
    }
}

fn run(command: &mut Command) -> Result<()> {
    let status = command.status().context("spawn command")?;
    if !status.success() {
        bail!("{} exited with {status}", command_name(command));
    }
    Ok(())
}

fn output(command: &mut Command) -> Result<String> {
    let output = command.output().context("spawn command")?;
    if !output.status.success() {
        bail!("{} exited with {}", command_name(command), output.status);
    }
    String::from_utf8(output.stdout).context("command output was not UTF-8")
}

fn command_name(command: &Command) -> String {
    command.get_program().to_string_lossy().into_owned()
}
