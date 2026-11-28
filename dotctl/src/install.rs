use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

pub const MANIFEST: &str = ".dotfiles-deployed";
const BACKUP_PREFIX: &str = ".dotfiles-backup-";

/// Deploy `etc/` into `home`, backing up conflicts and pruning files that
/// disappeared from the repository.
pub fn install(repo_root: &Path, home: &Path) -> Result<()> {
    let etc = repo_root.join("etc");
    if !etc.is_dir() {
        bail!(
            "{} not found (run from a clone of this repo)",
            etc.display()
        );
    }
    fs::create_dir_all(home).with_context(|| format!("create {}", home.display()))?;

    let manifest_path = home.join(MANIFEST);
    let old_manifest = read_manifest(&manifest_path)?;
    let old_set: HashSet<&str> = old_manifest.iter().map(String::as_str).collect();
    let files = collect_files(&etc)?;
    let new_set: HashSet<&str> = files.iter().map(|(rel, _)| rel.as_str()).collect();

    let mut backup: Option<PathBuf> = None;
    let mut backed_up = 0usize;

    for (rel, source) in &files {
        let target = home.join(rel);

        if is_same_symlink(&target, source) {
            fs::remove_file(&target)
                .with_context(|| format!("remove symlink {}", target.display()))?;
            continue;
        }

        if path_exists(&target)
            && (!old_set.contains(rel.as_str()) || !same_content(source, &target))
        {
            let dir = backup_dir(home, &mut backup)?;
            move_file(&target, &dir.join(rel))?;
            backed_up += 1;
        }
    }

    for (rel, source) in &files {
        let target = home.join(rel);
        if path_exists(&target) && old_set.contains(rel.as_str()) && same_content(source, &target) {
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        if path_exists(&target) {
            fs::remove_file(&target).with_context(|| format!("replace {}", target.display()))?;
        }
        fs::copy(source, &target)
            .with_context(|| format!("copy {} -> {}", source.display(), target.display()))?;
    }

    for rel in &old_manifest {
        if new_set.contains(rel.as_str()) {
            continue;
        }
        let target = home.join(rel);
        if !path_exists(&target) {
            continue;
        }
        let dir = backup_dir(home, &mut backup)?;
        move_file(&target, &dir.join("pruned").join(rel))?;
        println!("pruned {rel} (no longer in etc/)");
    }

    write_manifest(&manifest_path, &files)?;

    if backed_up > 0 {
        println!(
            "backed up {backed_up} existing file(s) -> {}",
            backup.as_ref().expect("backup dir exists").display()
        );
    }
    println!(
        "deployed {} file(s) into $HOME (copies; manifest: {})",
        files.len(),
        manifest_path.display()
    );
    Ok(())
}

/// Undo the last deployment, restoring conflicts and pruned files.
pub fn restore(home: &Path, dir: Option<PathBuf>) -> Result<()> {
    let manifest_path = home.join(MANIFEST);
    let manifest = read_manifest(&manifest_path)?;
    let restore_dir = match dir {
        Some(dir) => {
            if !dir.is_dir() {
                bail!("backup dir not found: {}", dir.display());
            }
            Some(dir)
        }
        None => newest_backup(home),
    };

    if !manifest_path.exists() && restore_dir.is_none() {
        bail!(
            "no manifest ({}) and no backup dir",
            manifest_path.display()
        );
    }

    let clean_restore = restore_dir.is_none();
    if clean_restore {
        println!("no backup dir (clean install); removing deployed files only");
    }

    let mut restored = 0usize;
    if !manifest.is_empty() {
        for rel in &manifest {
            let target = home.join(rel);
            if let Some(dir) = &restore_dir {
                let source = dir.join(rel);
                if path_exists(&source) {
                    replace_with(&source, &target)?;
                    restored += 1;
                    continue;
                }
            }
            remove_file(&target)?;
        }
    }

    if let Some(dir) = &restore_dir {
        if !manifest_path.exists() {
            println!("warning: no manifest; restoring everything in the backup");
            for (rel, source) in collect_files(dir)? {
                if rel.starts_with("pruned/") {
                    continue;
                }
                replace_with(&source, &home.join(rel))?;
                restored += 1;
            }
        }

        let pruned = dir.join("pruned");
        if pruned.is_dir() {
            for (rel, source) in collect_files(&pruned)? {
                replace_with(&source, &home.join(rel))?;
                restored += 1;
            }
        }
    }

    let marker = home.join(".dotfiles-cli-version");
    if marker.exists() {
        remove_file(&home.join(".local/bin/dotctl"))?;
        remove_file(&marker)?;
        println!("removed the dotctl binary installed by this repo");
    }

    remove_file(&manifest_path)?;

    if clean_restore {
        println!("removed deployed files (there was no backup to restore)");
    } else {
        println!(
            "restored {restored} file(s) from {} (deployed files removed)",
            restore_dir.expect("backup dir").display()
        );
    }
    Ok(())
}

fn backup_dir(home: &Path, state: &mut Option<PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = state {
        return Ok(dir.clone());
    }
    let dir = unique_backup_dir(home);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    *state = Some(dir.clone());
    Ok(dir)
}

fn unique_backup_dir(home: &Path) -> PathBuf {
    let stamp = timestamp();
    let base = format!("{BACKUP_PREFIX}{stamp}");
    let mut candidate = home.join(&base);
    let mut index = 1;
    while candidate.exists() {
        candidate = home.join(format!("{base}-{index}"));
        index += 1;
    }
    candidate
}

fn newest_backup(home: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(home).ok()?;
    entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(BACKUP_PREFIX)
        })
        .map(|entry| entry.path())
        .max_by(|a, b| a.file_name().cmp(&b.file_name()))
}

fn read_manifest(path: &Path) -> Result<Vec<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err).with_context(|| format!("read {}", path.display())),
    }
}

fn write_manifest(path: &Path, files: &[(String, PathBuf)]) -> Result<()> {
    let mut text = String::new();
    for (rel, _) in files {
        text.push_str(rel);
        text.push('\n');
    }
    fs::write(path, text).with_context(|| format!("write {}", path.display()))
}

fn collect_files(root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut files = Vec::new();
    walk(root, root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

fn walk(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(root, &path, files)?;
        } else if kind.is_file() || kind.is_symlink() {
            let rel = path
                .strip_prefix(root)
                .with_context(|| format!("strip prefix {}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            files.push((rel, path));
        }
    }
    Ok(())
}

fn is_same_symlink(target: &Path, source: &Path) -> bool {
    let is_link = fs::symlink_metadata(target)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false);
    if !is_link {
        return false;
    }
    fs::canonicalize(target).ok() == fs::canonicalize(source).ok()
}

fn same_content(a: &Path, b: &Path) -> bool {
    match (fs::read(a), fs::read(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn move_file(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    if path_exists(target) {
        remove_file(target)?;
    }
    fs::rename(source, target)
        .with_context(|| format!("move {} -> {}", source.display(), target.display()))
}

fn replace_with(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    if path_exists(target) {
        remove_file(target)?;
    }
    fs::rename(source, target)
        .with_context(|| format!("restore {} -> {}", source.display(), target.display()))
}

fn remove_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("remove {}", path.display())),
    }
}

fn timestamp() -> String {
    // SAFETY: localtime_r writes into the supplied tm and does not retain it.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return format!("{}", now);
        }
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "dotctl-install-{}-{}-{name}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(dir.join("repo/etc/.config")).unwrap();
        fs::create_dir_all(dir.join("home")).unwrap();
        dir
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn installs_backs_up_and_restores() {
        let root = scratch("basic");
        let repo = root.join("repo");
        let home = root.join("home");
        write(&repo, "etc/.bashrc", "new bashrc\n");
        write(&repo, "etc/.config/hypr/hyprland.lua", "new hypr\n");
        write(&home, ".config/hypr/hyprland.lua", "old hypr\n");

        install(&repo, &home).unwrap();
        assert_eq!(
            fs::read_to_string(home.join(".bashrc")).unwrap(),
            "new bashrc\n"
        );
        let manifest = fs::read_to_string(home.join(MANIFEST)).unwrap();
        assert!(manifest.contains(".bashrc"));

        let backup = newest_backup(&home).unwrap();
        assert_eq!(
            fs::read_to_string(backup.join(".config/hypr/hyprland.lua")).unwrap(),
            "old hypr\n"
        );

        restore(&home, None).unwrap();
        assert!(!home.join(".bashrc").exists());
        assert_eq!(
            fs::read_to_string(home.join(".config/hypr/hyprland.lua")).unwrap(),
            "old hypr\n"
        );
        assert!(!home.join(MANIFEST).exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prunes_removed_files_and_restores_them() {
        let root = scratch("prune");
        let repo = root.join("repo");
        let home = root.join("home");
        write(&repo, "etc/.old", "old script\n");
        write(&repo, "etc/.keep", "keep\n");

        install(&repo, &home).unwrap();
        assert!(home.join(".old").exists());

        fs::remove_file(repo.join("etc/.old")).unwrap();
        install(&repo, &home).unwrap();
        assert!(!home.join(".old").exists());

        restore(&home, None).unwrap();
        assert_eq!(
            fs::read_to_string(home.join(".old")).unwrap(),
            "old script\n"
        );
        assert!(!home.join(".keep").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
