use std::time::Duration;

use anyhow::{Context, Result};
use zbus::Connection;

pub const INTERFACE: &str = "net.hadess.PowerProfiles";
pub const PATH: &str = "/net/hadess/PowerProfiles";

pub async fn connect() -> Result<Connection> {
    zbus::connection::Builder::system()
        .method_timeout(Duration::from_millis(800))
        .build()
        .await
        .context("connect to the system bus")
}

pub async fn active_profile(connection: &Connection) -> Result<String> {
    let proxy = zbus::Proxy::new(connection, INTERFACE, PATH, INTERFACE)
        .await
        .context("create PowerProfiles proxy")?;
    proxy
        .get_property::<String>("ActiveProfile")
        .await
        .context("read ActiveProfile")
}

pub async fn set_profile(connection: &Connection, profile: &str) -> Result<()> {
    connection
        .call_method(
            Some(INTERFACE),
            PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &(
                INTERFACE,
                "ActiveProfile",
                zbus::Variant::from(profile.to_owned()),
            ),
        )
        .await
        .with_context(|| format!("set ActiveProfile to {profile}"))?;
    Ok(())
}

pub fn active_profile_now() -> Result<String> {
    zbus::block_on(async {
        let connection = connect().await?;
        active_profile(&connection).await
    })
}
