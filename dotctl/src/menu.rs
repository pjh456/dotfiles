use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::{notify, rofi};

const CLIPBOARD_THEME: &str = r#"
  window { width: 720px; }
  listview { columns: 1; lines: 12; }
  element { padding: 8px 12px; }
"#;

const EMOJI_FORMAT: &str =
    r##"<span size="x-large">{emoji}</span>   <span foreground="#bac2de">{name}</span>"##;

const EMOJI_THEME: &str = r#"
    window { width: 560px; }
    mainbox { children: [inputbar, listview]; }
    listview { lines: 10; columns: 1; spacing: 4px; }
    element { padding: 8px 12px; border-radius: 10px; }
    element selected.normal { background-color: #313244cc; text-color: #cdd6f4; border: 1px; border-color: #89b4fa66; }
"#;

const POWER_THEME: &str = r#"
    window { width: 520px; border-radius: 16px; border: 2px; border-color: #cba6f766; background-color: #181825e0; }
    mainbox { children: [listview]; padding: 16px; }
    listview { columns: 5; lines: 1; spacing: 12px; layout: vertical; border: 0px; }
    element { children: [element-text]; padding: 22px 0px; border: 0px; border-radius: 12px; background-color: #31324499; }
    element normal.normal { background-color: #31324499; }
    element alternate.normal { background-color: #31324499; }
    element selected.normal { background-image: linear-gradient(135deg, #89b4fa, #cba6f7); text-color: #11111b; }
    element-text { font: "JetBrainsMono Nerd Font Propo 24"; horizontal-align: 0.5; vertical-align: 0.5; }
"#;

pub fn clipboard() -> Result<()> {
    if rofi::close_if_open() {
        return Ok(());
    }

    let list = Command::new("cliphist")
        .arg("list")
        .output()
        .context("run cliphist list")?;
    if !list.status.success() || list.stdout.is_empty() {
        return Ok(());
    }

    let args = vec![
        "-dmenu".to_owned(),
        "-p".to_owned(),
        " Clipboard".to_owned(),
        "-display-columns".to_owned(),
        "2".to_owned(),
        "-theme-str".to_owned(),
        CLIPBOARD_THEME.to_owned(),
    ];
    let output = rofi::run_with_input(&args, &list.stdout)?;
    let item = trim_newline(&output.stdout);
    if item.is_empty() {
        return Ok(());
    }

    let mut child = Command::new("cliphist")
        .arg("decode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("run cliphist decode")?;
    child
        .stdin
        .take()
        .context("cliphist stdin was not piped")?
        .write_all(&item)
        .context("write cliphist stdin")?;
    let decoded = child
        .wait_with_output()
        .context("wait for cliphist decode")?;
    if !decoded.status.success() || decoded.stdout.is_empty() {
        return Ok(());
    }

    notify::send("Clipboard", "Copied, press Ctrl+V to paste", "edit-paste");
    // The old pipeline used command substitution (which strips the trailing
    // newline) and then `echo`, so the clipboard always ends with one newline.
    let mut data = trim_newline(&decoded.stdout);
    data.push(b'\n');
    copy(&data, true)
}

pub fn emoji(paste: bool) -> Result<()> {
    if rofi::close_if_open() {
        return Ok(());
    }

    if paste {
        let args = emoji_args("stdout");
        let output = rofi::run_output(&args)?;
        let emoji = trim_newline(&output.stdout);
        if emoji.is_empty() {
            return Ok(());
        }
        copy(&emoji, false)?;
        if have("wtype") {
            let status = Command::new("wtype")
                .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
                .status()
                .context("run wtype")?;
            if !status.success() {
                bail!("wtype failed with {status}");
            }
        } else {
            notify::send(
                "Emoji",
                "Copied to the clipboard — press Ctrl+V",
                "face-smile",
            );
        }
        return Ok(());
    }

    let mode = if have("wtype") { "insert" } else { "copy" };
    rofi::run(&emoji_args(mode))
}

pub fn power() -> Result<()> {
    if rofi::close_if_open() {
        return Ok(());
    }

    let items = format!(
        "{}\n{}\n{}\n{}\n{}",
        "\u{f023}", "\u{f26c}", "\u{f426}", "\u{f021}", "\u{f011}"
    );
    let args = vec![
        "-dmenu".to_owned(),
        "-format".to_owned(),
        "i".to_owned(),
        "-p".to_owned(),
        " Power".to_owned(),
        "-theme-str".to_owned(),
        POWER_THEME.to_owned(),
    ];
    let output = rofi::run_with_input(&args, items.as_bytes())?;
    let selected = String::from_utf8_lossy(&output.stdout);
    let selected = selected.trim();

    match selected {
        "0" => run_command("hyprlock", &[])?,
        "1" => run_command("systemctl", &["suspend"])?,
        "2" => run_command("hyprctl", &["eval", "hl.dsp.exit()"])?,
        "3" => run_command("reboot", &[])?,
        "4" => run_command("poweroff", &[])?,
        "" => {}
        other => bail!("unexpected power-menu index: {other}"),
    }

    Ok(())
}

pub fn launcher() -> Result<()> {
    rofi::toggle(&["-show".to_owned(), "drun".to_owned()])
}

fn emoji_args(mode: &str) -> Vec<String> {
    vec![
        "-modi".to_owned(),
        "emoji".to_owned(),
        "-show".to_owned(),
        "emoji".to_owned(),
        "-emoji-mode".to_owned(),
        mode.to_owned(),
        "-emoji-format".to_owned(),
        EMOJI_FORMAT.to_owned(),
        "-p".to_owned(),
        "  Emoji".to_owned(),
        "-theme-str".to_owned(),
        EMOJI_THEME.to_owned(),
    ]
}

fn trim_newline(bytes: &[u8]) -> Vec<u8> {
    let mut end = bytes.len();
    while end > 0 && matches!(bytes[end - 1], b'\n' | b'\r') {
        end -= 1;
    }
    bytes[..end].to_vec()
}

fn copy(data: &[u8], paste_once: bool) -> Result<()> {
    let mut command = Command::new("wl-copy");
    if paste_once {
        command.arg("--paste-once");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .spawn()
        .context("spawn wl-copy")?;
    child
        .stdin
        .take()
        .context("wl-copy stdin was not piped")?
        .write_all(data)
        .context("write wl-copy stdin")?;
    let status = child.wait().context("wait for wl-copy")?;
    if !status.success() {
        bail!("wl-copy failed with {status}");
    }
    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("run {program}"))?;
    if !status.success() {
        bail!("{program} failed with {status}");
    }
    Ok(())
}

fn have(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .any(|candidate| candidate.is_file())
}
