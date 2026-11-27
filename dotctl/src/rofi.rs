use std::io::Write;
use std::process::{Command, Output, Stdio};

use anyhow::{Context, Result};

pub fn is_open() -> bool {
    Command::new("pgrep")
        .args(["-x", "rofi"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Rofi refuses to start a second instance, so a second press should close the
/// open menu. Returns true when an existing menu was closed.
pub fn close_if_open() -> bool {
    if !is_open() {
        return false;
    }
    let _ = Command::new("pkill").args(["-x", "rofi"]).status();
    true
}

pub fn toggle(args: &[String]) -> Result<()> {
    if close_if_open() {
        return Ok(());
    }
    run(args)
}

pub fn run(args: &[String]) -> Result<()> {
    Command::new("rofi")
        .args(args)
        .status()
        .context("spawn rofi")?;
    Ok(())
}

pub fn run_with_input(args: &[String], input: &[u8]) -> Result<Output> {
    let mut child = Command::new("rofi")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("spawn rofi")?;

    child
        .stdin
        .take()
        .context("rofi stdin was not piped")?
        .write_all(input)
        .context("write rofi stdin")?;

    child.wait_with_output().context("wait for rofi")
}

pub fn run_output(args: &[String]) -> Result<Output> {
    Command::new("rofi").args(args).output().context("run rofi")
}
