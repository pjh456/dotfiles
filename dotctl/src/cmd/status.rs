use anyhow::Result;

use crate::cli::StatusKind;
use crate::sysfs;
use crate::waybar::WaybarOutput;

pub fn run(kind: StatusKind) -> Result<()> {
    match kind {
        StatusKind::Temp => temp().print(),
    }
    Ok(())
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
