use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// 1x1 opaque black RGBA PNG. Kept in the binary so `unset` does not depend on
/// ImageMagick just to create a solid wallpaper.
const BLACK_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x60, 0x60, 0x60, 0xf8,
    0x0f, 0x00, 0x01, 0x04, 0x01, 0x00, 0x80, 0xbb, 0xd1, 0x5b, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
    0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

pub fn set(path: &Path) -> Result<()> {
    let path = fs::canonicalize(path)
        .with_context(|| format!("wallpaper not found: {}", path.display()))?;
    if !path.is_file() {
        bail!("not a regular file: {}", path.display());
    }

    hyprpaper_wallpaper(&path)?;
    remember(&path)?;
    Ok(())
}

pub fn unset() -> Result<()> {
    let black = Path::new("/tmp/hyprpaper_black.png");
    if !black.exists() {
        fs::write(black, BLACK_PNG).context("write /tmp/hyprpaper_black.png")?;
    }

    hyprpaper_wallpaper(black)?;

    let cache = cache_dir()?;
    let _ = fs::remove_file(cache.join("current_wallpaper"));
    Ok(())
}

fn hyprpaper_wallpaper(path: &Path) -> Result<()> {
    let command = format!("wallpaper ,{}", path.display());
    let status = Command::new("hyprctl")
        .args(["hyprpaper", &command])
        .status()
        .context("spawn hyprctl")?;
    if !status.success() {
        bail!("hyprctl hyprpaper failed with {status}");
    }
    Ok(())
}

fn remember(path: &Path) -> Result<()> {
    let cache = cache_dir()?;
    fs::create_dir_all(&cache).with_context(|| format!("create {}", cache.display()))?;
    let link = cache.join("current_wallpaper");
    let _ = fs::remove_file(&link);
    symlink(path, &link).with_context(|| format!("link {}", link.display()))?;
    Ok(())
}

fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(dir));
    }
    let home = env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".cache"))
}
