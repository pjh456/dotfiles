use std::fs;
use std::path::Path;

/// AC is online when any non-battery supply reports `online = 1`.
///
/// hypridle calls this and expects the shell-script contract: exit 0 on
/// battery (lock/suspend), exit 1 on AC (leave the session alone).
pub fn ac_online() -> bool {
    ac_online_in(Path::new("/sys/class/power_supply"))
}

fn ac_online_in(root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        let supply_type = fs::read_to_string(dir.join("type"))
            .map(|value| value.trim().to_owned())
            .unwrap_or_default();
        if supply_type == "Battery" {
            continue;
        }
        let online = fs::read_to_string(dir.join("online")).unwrap_or_default();
        if online.trim() == "1" {
            return true;
        }
    }

    false
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
        let dir = std::env::temp_dir().join(format!("dotctl-power-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn supply(root: &Path, name: &str, kind: &str, online: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("type"), kind).unwrap();
        fs::write(dir.join("online"), online).unwrap();
    }

    #[test]
    fn mains_online_means_ac() {
        let root = scratch();
        supply(&root, "BAT0", "Battery", "0");
        supply(&root, "AC", "Mains", "1");
        assert!(ac_online_in(&root));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn battery_only_means_discharging() {
        let root = scratch();
        supply(&root, "BAT0", "Battery", "1");
        supply(&root, "AC", "Mains", "0");
        assert!(!ac_online_in(&root));
        fs::remove_dir_all(root).unwrap();
    }
}
