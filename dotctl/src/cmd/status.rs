use anyhow::Result;

use crate::bluez;
use crate::cli::StatusKind;
use crate::sysfs;
use crate::waybar::WaybarOutput;

pub fn run(kind: StatusKind) -> Result<()> {
    match kind {
        StatusKind::Temp => temp().print(),
        StatusKind::Bt => bt().print(),
    }
    Ok(())
}

const BT_ICON: &str = "\u{f294}";

/// Bluetooth status, matching the old `bt-toggle status` JSON contract.
pub fn bt() -> WaybarOutput {
    let off = || WaybarOutput::new(format!("{BT_ICON} off"), "off").with_tooltip("Bluetooth off");

    if !bluez::service_active() {
        return off();
    }

    let snapshot = match bluez::snapshot() {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("dotctl: bluetooth status: {err:#}");
            return off();
        }
    };

    if !snapshot.powered {
        return off();
    }

    match snapshot.devices.first() {
        Some(alias) => {
            WaybarOutput::new(format!("{BT_ICON} {alias}"), "connected").with_tooltip(alias)
        }
        None => WaybarOutput::new(format!("{BT_ICON} on"), "on").with_tooltip("Bluetooth on"),
    }
}

/// CPU temperature, matching the old `temperature` script's JSON contract.
pub fn temp() -> WaybarOutput {
    match sysfs::cpu_temp() {
        Some(temp) => {
            let (class, icon) = temp_style(temp);
            WaybarOutput::new(format!("{icon} {temp}°C"), class)
        }
        None => WaybarOutput::new("N/A", "normal"),
    }
}

fn temp_style(temp: i32) -> (&'static str, &'static str) {
    match temp {
        i32::MIN..=44 => ("normal", "\u{f2cb}"),
        45..=59 => ("warm", "\u{f2c9}"),
        60..=74 => ("hot", "\u{f2c8}"),
        75..=84 => ("very-hot", "\u{f2c7}"),
        _ => ("critical", "\u{e231}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_match_the_shell_script() {
        assert_eq!(temp_style(44), ("normal", "\u{f2cb}"));
        assert_eq!(temp_style(45), ("warm", "\u{f2c9}"));
        assert_eq!(temp_style(59), ("warm", "\u{f2c9}"));
        assert_eq!(temp_style(60), ("hot", "\u{f2c8}"));
        assert_eq!(temp_style(74), ("hot", "\u{f2c8}"));
        assert_eq!(temp_style(75), ("very-hot", "\u{f2c7}"));
        assert_eq!(temp_style(84), ("very-hot", "\u{f2c7}"));
        assert_eq!(temp_style(85), ("critical", "\u{e231}"));
    }

    #[test]
    fn json_field_names_match_waybar() {
        let output = WaybarOutput::new("x 51°C", "warm");
        assert_eq!(
            serde_json::to_string(&output).unwrap(),
            r#"{"text":"x 51°C","class":"warm"}"#
        );
    }
}
