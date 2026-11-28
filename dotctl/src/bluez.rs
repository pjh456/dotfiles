use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use zbus::OwnedObjectPath;
use zbus::fdo::ManagedObjects;

const SERVICE: &str = "bluetooth.service";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BtSnapshot {
    pub powered: bool,
    pub devices: Vec<String>,
}

/// Query BlueZ in one `GetManagedObjects` call.
pub fn snapshot() -> Result<BtSnapshot> {
    zbus::block_on(async {
        let connection = connect().await?;
        let objects = managed_objects(&connection).await?;
        Ok(parse_objects(&objects))
    })
}

/// The old script deliberately avoided D-Bus activation when the service was
/// stopped: keep that behaviour so a Waybar poll never starts Bluetooth.
pub fn service_active() -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", SERVICE])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn parse_objects(objects: &ManagedObjects) -> BtSnapshot {
    let mut snapshot = BtSnapshot::default();

    for interfaces in objects.values() {
        for (interface, properties) in interfaces {
            match interface.as_str() {
                "org.bluez.Adapter1" => {
                    if let Some(value) = properties.get("Powered") {
                        snapshot.powered = value.downcast_ref::<&bool>().copied().unwrap_or(false);
                    }
                }
                "org.bluez.Device1" => {
                    let connected = properties
                        .get("Connected")
                        .and_then(|value| value.downcast_ref::<&bool>().ok())
                        .copied()
                        .unwrap_or(false);
                    if !connected {
                        continue;
                    }
                    let alias = properties
                        .get("Alias")
                        .and_then(|value| value.downcast_ref::<&str>().ok())
                        .map(str::to_owned)
                        .or_else(|| {
                            properties
                                .get("Address")
                                .and_then(|value| value.downcast_ref::<&str>().ok())
                                .map(str::to_owned)
                        })
                        .unwrap_or_else(|| "device".to_owned());
                    snapshot.devices.push(alias);
                }
                _ => {}
            }
        }
    }

    snapshot.devices.sort();
    snapshot
}

/// Path of the first BlueZ adapter, if one is present.
pub fn adapter_path(objects: &ManagedObjects) -> Option<OwnedObjectPath> {
    objects.iter().find_map(|(path, interfaces)| {
        interfaces
            .keys()
            .any(|interface| interface.as_str() == "org.bluez.Adapter1")
            .then(|| path.clone())
    })
}

/// Power the adapter on. This is what `bluetoothctl power on` did, but through
/// the same connection that reads the state.
pub fn power_on() -> Result<()> {
    zbus::block_on(async {
        let connection = connect().await?;
        let objects = managed_objects(&connection).await?;
        let path = adapter_path(&objects).context("no bluetooth adapter found")?;

        connection
            .call_method(
                Some("org.bluez"),
                path.as_str(),
                Some("org.freedesktop.DBus.Properties"),
                "Set",
                &("org.bluez.Adapter1", "Powered", zbus::Variant::from(true)),
            )
            .await
            .context("power on the bluetooth adapter")?;

        Ok(())
    })
}

/// Start Bluetooth on demand, power the adapter on, and refresh Waybar.
pub fn toggle() -> Result<()> {
    // A soft rfkill block survives across boots and makes `bluetoothctl power
    // on` fail even though the service starts. The sudoers rule installed by
    // install.sh allows exactly this command without a password.
    if crate::daemon::have_cmd("rfkill") {
        match Command::new("sudo")
            .args(["-n", "rfkill", "unblock", "bluetooth"])
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => eprintln!("dotctl: rfkill unblock bluetooth failed with {status}"),
            Err(err) => eprintln!("dotctl: rfkill unblock bluetooth: {err}"),
        }
    }

    if !service_active() {
        let status = Command::new("sudo")
            .args(["-n", "systemctl", "start", SERVICE])
            .status()
            .context("spawn sudo systemctl start bluetooth.service")?;
        if !status.success() {
            bail!(
                "could not start {SERVICE}; the passwordless sudoers rule is missing \
                 (run install.sh without --no-sudo)"
            );
        }

        if !wait_until(service_active, 20, Duration::from_millis(100)) {
            bail!("bluetooth.service did not become active");
        }
    }

    if let Err(err) = power_on() {
        eprintln!("dotctl: could not power on the bluetooth adapter: {err:#}");
    } else if !wait_until(
        || snapshot().map(|snapshot| snapshot.powered).unwrap_or(false),
        15,
        Duration::from_millis(200),
    ) {
        eprintln!("dotctl: bluetooth adapter did not report Powered=true");
    }

    // Match the old script: open the manager even when there is no usable
    // adapter, so the user can see what is wrong (rfkill, no controller, ...).
    refresh_waybar();
    launch_blueman();
    refresh_waybar();
    Ok(())
}

async fn connect() -> Result<zbus::Connection> {
    zbus::connection::Builder::system()
        .method_timeout(Duration::from_millis(800))
        .build()
        .await
        .context("connect to the system bus")
}

async fn managed_objects(connection: &zbus::Connection) -> Result<ManagedObjects> {
    zbus::fdo::ObjectManagerProxy::builder(connection)
        .destination("org.bluez")
        .path("/")
        .build()
        .await?
        .get_managed_objects()
        .await
        .context("org.bluez GetManagedObjects")
}

fn wait_until(mut predicate: impl FnMut() -> bool, attempts: usize, delay: Duration) -> bool {
    for attempt in 0..attempts {
        if predicate() {
            return true;
        }
        if attempt + 1 < attempts {
            thread::sleep(delay);
        }
    }
    false
}

fn refresh_waybar() {
    let _ = Command::new("pkill").args(["-RTMIN+1", "waybar"]).status();
}

fn launch_blueman() {
    if crate::daemon::have_cmd("blueman-manager") {
        let _ = Command::new("blueman-manager").spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zbus::names::OwnedInterfaceName;
    use zbus::{OwnedObjectPath, OwnedVariant, Str};

    fn interface(name: &str) -> OwnedInterfaceName {
        name.try_into().unwrap()
    }

    fn path(name: &str) -> OwnedObjectPath {
        name.try_into().unwrap()
    }

    fn props(entries: &[(&str, OwnedVariant)]) -> HashMap<String, OwnedVariant> {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect()
    }

    #[test]
    fn powered_adapter_without_devices() {
        let mut objects = HashMap::new();
        objects.insert(
            path("/org/bluez/hci0"),
            HashMap::from([(
                interface("org.bluez.Adapter1"),
                props(&[("Powered", OwnedVariant::from(true))]),
            )]),
        );

        assert_eq!(
            parse_objects(&objects),
            BtSnapshot {
                powered: true,
                devices: vec![]
            }
        );
    }

    #[test]
    fn connected_devices_use_alias_and_are_sorted() {
        let mut objects = HashMap::new();
        objects.insert(
            path("/org/bluez/hci0"),
            HashMap::from([(
                interface("org.bluez.Adapter1"),
                props(&[("Powered", OwnedVariant::from(true))]),
            )]),
        );
        for (index, alias) in ["Keyboard", "Headset"].iter().enumerate() {
            objects.insert(
                path(&format!("/org/bluez/hci0/dev_{index}")),
                HashMap::from([(
                    interface("org.bluez.Device1"),
                    props(&[
                        ("Connected", OwnedVariant::from(true)),
                        ("Alias", OwnedVariant::from(Str::from(*alias))),
                    ]),
                )]),
            );
        }

        assert_eq!(
            parse_objects(&objects),
            BtSnapshot {
                powered: true,
                devices: vec!["Headset".to_owned(), "Keyboard".to_owned()]
            }
        );
    }

    #[test]
    fn disconnected_devices_are_ignored() {
        let mut objects = HashMap::new();
        objects.insert(
            path("/org/bluez/hci0/dev_1"),
            HashMap::from([(
                interface("org.bluez.Device1"),
                props(&[("Connected", OwnedVariant::from(false))]),
            )]),
        );

        assert_eq!(parse_objects(&objects), BtSnapshot::default());
    }
}
