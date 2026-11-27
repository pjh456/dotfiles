use std::fs;
use std::path::{Path, PathBuf};

/// Read a one-line sysfs file and trim it.
pub fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

fn read_i64(path: impl AsRef<Path>) -> Option<i64> {
    read_trim(path)?.parse().ok()
}

fn sensor_input(dir: &Path, label: &str) -> Option<i64> {
    // hwmon exposes tempN_label + tempN_input pairs. Walk the labels so we
    // pick the exact sensor the old `sensors | awk` pipeline used to select.
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_str()?;
        if !name.starts_with("temp") || !name.ends_with("_label") {
            continue;
        }
        if read_trim(entry.path()).as_deref() != Some(label) {
            continue;
        }
        let input = name.trim_end_matches("_label").to_owned() + "_input";
        return read_i64(dir.join(input));
    }
    None
}

fn first_temp_input(dir: &Path) -> Option<i64> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_str()?;
        if name.starts_with("temp")
            && name.ends_with("_input")
            && let Some(value) = read_i64(entry.path())
        {
            return Some(value);
        }
    }
    None
}

fn hwmon_dirs(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = read_trim(path.join("name")) else {
            continue;
        };
        out.push((name, path));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Read the CPU package temperature in whole degrees Celsius.
///
/// This mirrors the old pipeline `sensors | awk '/Package id 0/...'` on the
/// machines this repo runs on, but without spawning `sensors` or `awk`.
pub fn cpu_temp() -> Option<i32> {
    cpu_temp_from(Path::new("/sys/class/hwmon"))
}

fn cpu_temp_from(root: &Path) -> Option<i32> {
    let dirs = hwmon_dirs(root);

    let find = |name: &str| {
        dirs.iter()
            .find(|(n, _)| n == name)
            .map(|(_, path)| path.clone())
    };

    // Intel: coretemp exposes "Package id 0".
    if let Some(path) = find("coretemp")
        && let Some(value) = sensor_input(&path, "Package id 0")
    {
        return milli_to_celsius(value);
    }

    // AMD: k10temp/zenpower expose Tctl (and sometimes Tdie).
    for name in ["k10temp", "zenpower"] {
        if let Some(path) = find(name) {
            for label in ["Tctl", "Tdie"] {
                if let Some(value) = sensor_input(&path, label) {
                    return milli_to_celsius(value);
                }
            }
            if let Some(value) = first_temp_input(&path) {
                return milli_to_celsius(value);
            }
        }
    }

    // Last resort: the ACPI thermal zone.
    if let Some(path) = find("acpitz")
        && let Some(value) = first_temp_input(&path)
    {
        return milli_to_celsius(value);
    }

    // Some platforms (ARM, VMs) only expose a generic CPU thermal zone.
    for (name, path) in &dirs {
        if (name == "cpu_thermal" || name == "soc_thermal")
            && let Some(value) = first_temp_input(path)
        {
            return milli_to_celsius(value);
        }
    }

    None
}

fn milli_to_celsius(milli: i64) -> Option<i32> {
    // Ignore "sensor not populated" values; hwmon uses 0 for those and the
    // occasional driver reports negative garbage.
    if !(1_000..200_000).contains(&milli) {
        return None;
    }
    Some((milli / 1_000) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "dotctl-test-{}-{}-{name}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn add_hwmon(root: &Path, index: usize, name: &str, temp: &str) -> PathBuf {
        let dir = root.join(format!("hwmon{index}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("name"), name).unwrap();
        fs::write(dir.join("temp1_input"), temp).unwrap();
        dir
    }

    #[test]
    fn coretemp_package_id_wins_over_acpitz() {
        let root = scratch("coretemp");
        add_hwmon(&root, 0, "acpitz", "45000\n");
        let coretemp = add_hwmon(&root, 1, "coretemp", "51000\n");
        fs::write(coretemp.join("temp1_label"), "Package id 0\n").unwrap();
        fs::write(coretemp.join("temp2_label"), "Core 0\n").unwrap();

        assert_eq!(cpu_temp_from(&root), Some(51));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn k10temp_tctl_is_used_when_coretemp_is_absent() {
        let root = scratch("k10temp");
        let dir = add_hwmon(&root, 0, "k10temp", "62000\n");
        fs::write(dir.join("temp1_label"), "Tctl\n").unwrap();

        assert_eq!(cpu_temp_from(&root), Some(62));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn zero_and_negative_values_are_ignored() {
        let root = scratch("invalid");
        add_hwmon(&root, 0, "acpitz", "0\n");
        assert_eq!(cpu_temp_from(&root), None);
        fs::remove_dir_all(root).unwrap();
    }
}
