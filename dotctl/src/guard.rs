use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use inotify::{Inotify, WatchMask};

use crate::daemon;

pub fn run(once: bool, state_dir: Option<PathBuf>) -> Result<()> {
    let state = match state_dir {
        Some(dir) => dir,
        None => default_state_dir()?,
    };
    fs::create_dir_all(&state).with_context(|| format!("create {}", state.display()))?;
    daemon::install_exit_handler()?;
    let runtime = runtime_dir();

    if once {
        let log = newest_log(&runtime).context("no Hyprland log found")?;
        let sig = instance_signature(&log).context("cannot read the Hyprland instance")?;
        if ping(&runtime, &sig) {
            println!("Hyprland alive ({sig})");
            return Ok(());
        }
        bail!("Hyprland did not answer on {sig}");
    }

    let Some(_lock) = lock(&state.join(".lock"))? else {
        return Ok(());
    };
    let mirror_state = state.clone();
    let mirror_runtime = runtime.clone();
    thread::spawn(move || mirror_logs(&mirror_runtime, &mirror_state));

    let mut fails = 0;
    let mut last_capture = 0u64;

    loop {
        thread::sleep(Duration::from_secs(1));

        let Some(log) = newest_log(&runtime) else {
            fails = 0;
            continue;
        };
        let Some(sig) = instance_signature(&log) else {
            fails = 0;
            continue;
        };

        if ping(&runtime, &sig) {
            fails = 0;
            continue;
        }

        fails += 1;
        let now = now_secs();
        if fails >= 2 && now.saturating_sub(last_capture) > 60 {
            last_capture = now;
            if let Err(err) = capture(&state, &log, &sig) {
                eprintln!("dotctl: guard: capture failed: {err:#}");
            }
            fails = 0;
        }
    }
}

fn mirror_logs(runtime: &Path, state: &Path) {
    let mut inotify = match Inotify::init() {
        Ok(inotify) => inotify,
        Err(err) => {
            eprintln!("dotctl: guard: inotify init: {err}");
            return;
        }
    };

    let mut watched = HashSet::new();
    let root = runtime.join("hypr");
    let _ = inotify
        .watches()
        .add(&root, WatchMask::CREATE | WatchMask::MOVED_TO);
    ensure_instance_watches(&mut inotify, runtime, &mut watched);
    copy_newest_log(runtime, state);

    let mut buffer = [0u8; 4096];
    loop {
        match inotify.read_events_blocking(&mut buffer) {
            Ok(_) => {
                ensure_instance_watches(&mut inotify, runtime, &mut watched);
                copy_newest_log(runtime, state);
            }
            Err(err) => {
                eprintln!("dotctl: guard: inotify read: {err}");
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}

fn ensure_instance_watches(inotify: &mut Inotify, runtime: &Path, watched: &mut HashSet<PathBuf>) {
    let root = runtime.join("hypr");
    let Ok(entries) = fs::read_dir(&root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || watched.contains(&path) {
            continue;
        }
        if inotify
            .watches()
            .add(
                &path,
                WatchMask::MODIFY | WatchMask::CREATE | WatchMask::MOVED_TO,
            )
            .is_ok()
        {
            watched.insert(path);
        }
    }
}

fn copy_newest_log(runtime: &Path, state: &Path) {
    let Some(log) = newest_log(runtime) else {
        return;
    };
    let dest = state.join("current.log");
    let changed = match (fs::metadata(&log), fs::metadata(&dest)) {
        (Ok(src), Ok(dst)) => src.len() != dst.len() || src.modified().ok() != dst.modified().ok(),
        _ => true,
    };
    if changed {
        let _ = fs::copy(&log, &dest);
    }
}

fn capture(state: &Path, log: &Path, sig: &str) -> Result<()> {
    let stamp = stamp();
    let out = state.join(format!("hang-{stamp}.txt"));
    let pid = hyprland_pid();

    let mut report = String::new();
    let now = Command::new("date")
        .arg("-Is")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default();
    report.push_str(&format!("== 时间: {now}\n"));
    report.push_str(&format!(
        "== Hyprland pid: {}   instance: {sig}\n",
        pid.map(|pid| pid.to_string())
            .unwrap_or_else(|| "?".to_owned())
    ));
    if let Ok(output) = Command::new("uptime").output() {
        report.push_str(&format!(
            "== uptime/load: {}\n",
            String::from_utf8_lossy(&output.stdout).trim()
        ));
    }
    report.push('\n');

    if let Some(pid) = pid {
        report.push_str("== 主线程 stat (state= D 不可中断 / R 运行 / S 睡眠):\n");
        if let Some(stat) = proc_stat(pid) {
            report.push_str(&format!(
                "   state={}  utime={}  stime={}  threads={}\n",
                stat.state, stat.utime, stat.stime, stat.threads
            ));
        }
        report.push('\n');

        report.push_str("== 各线程: comm / state / wchan / syscall(号+参数)\n");
        if let Ok(entries) = fs::read_dir(format!("/proc/{pid}/task")) {
            for entry in entries.flatten() {
                let tid = entry.file_name().to_string_lossy().into_owned();
                let comm = read_trim(entry.path().join("comm")).unwrap_or_default();
                let state = proc_stat_from(&entry.path().join("stat"))
                    .map(|stat| stat.state.to_owned())
                    .unwrap_or_default();
                let wchan = read_trim(entry.path().join("wchan")).unwrap_or_default();
                let syscall = read_trim(entry.path().join("syscall")).unwrap_or_default();
                report.push_str(&format!(
                    "   #{tid:<7} {comm:<18} state={state}  wchan={wchan}\n"
                ));
                report.push_str(&format!("            syscall: {syscall}\n"));
            }
        }
        report.push('\n');

        report.push_str("== 打开的关键 fd:\n");
        if let Ok(entries) = fs::read_dir(format!("/proc/{pid}/fd")) {
            for entry in entries.flatten() {
                let Ok(target) = fs::read_link(entry.path()) else {
                    continue;
                };
                let target = target.to_string_lossy();
                if target.contains("dri") || target.contains("wayland") || target.contains("socket")
                {
                    report.push_str(&format!(
                        "   {} -> {target}\n",
                        entry.file_name().to_string_lossy()
                    ));
                }
            }
        }
        report.push('\n');

        report.push_str("== gdb 全线程 backtrace (ptrace_scope=1 时会失败):\n");
        if let Ok(output) = Command::new("gdb")
            .args([
                "-p",
                &pid.to_string(),
                "-batch",
                "-ex",
                "thread apply all bt",
            ])
            .output()
        {
            for line in String::from_utf8_lossy(&output.stdout)
                .lines()
                .chain(String::from_utf8_lossy(&output.stderr).lines())
                .take(500)
            {
                report.push_str(line);
                report.push('\n');
            }
        }
    }

    fs::write(&out, report).with_context(|| format!("write {}", out.display()))?;
    let _ = fs::copy(log, state.join(format!("log-at-hang-{stamp}.log")));
    Ok(())
}

fn ping(runtime: &Path, sig: &str) -> bool {
    let socket = runtime.join("hypr").join(sig).join(".socket.sock");
    let Ok(mut stream) = UnixStream::connect(&socket) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    if stream.write_all(b"j/version\n").is_err() {
        return false;
    }
    let mut reply = String::new();
    stream.read_to_string(&mut reply).is_ok() && !reply.trim().is_empty()
}

fn newest_log(runtime: &Path) -> Option<PathBuf> {
    let hypr = runtime.join("hypr");
    let mut best: Option<(SystemTime, PathBuf)> = None;

    if let Some(sig) = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE") {
        let path = hypr.join(sig).join("hyprland.log");
        if path.is_file() {
            return Some(path);
        }
    }

    let entries = fs::read_dir(hypr).ok()?;
    for entry in entries.flatten() {
        let path = entry.path().join("hyprland.log");
        if !path.is_file() {
            continue;
        }
        let modified = fs::metadata(&path)
            .and_then(|meta| meta.modified())
            .unwrap_or(UNIX_EPOCH);
        if best
            .as_ref()
            .map(|(time, _)| modified > *time)
            .unwrap_or(true)
        {
            best = Some((modified, path));
        }
    }
    best.map(|(_, path)| path)
}

fn instance_signature(log: &Path) -> Option<String> {
    log.parent()?
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn default_state_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("hyprland-guard"))
}

fn runtime_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir);
    }
    // SAFETY: getuid has no preconditions and cannot fail.
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/run/user/{uid}"))
}

fn lock(path: &Path) -> Result<Option<File>> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    // SAFETY: flock only needs a valid fd, which the owned File provides.
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        return Ok(None);
    }
    Ok(Some(file))
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn stamp() -> String {
    // SAFETY: localtime_r writes into the supplied tm and does not retain it.
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return now_secs().to_string();
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

fn hyprland_pid() -> Option<u32> {
    let entries = fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };
        if read_trim(entry.path().join("comm")).as_deref() == Some("Hyprland") {
            return Some(pid);
        }
    }
    None
}

struct Stat {
    state: String,
    utime: String,
    stime: String,
    threads: String,
}

fn proc_stat(pid: u32) -> Option<Stat> {
    proc_stat_from(&PathBuf::from(format!("/proc/{pid}/stat")))
}

fn proc_stat_from(path: &Path) -> Option<Stat> {
    let raw = fs::read_to_string(path).ok()?;
    let fields: Vec<&str> = raw.split_whitespace().collect();
    Some(Stat {
        state: fields.get(2)?.to_string(),
        utime: fields.get(13)?.to_string(),
        stime: fields.get(14)?.to_string(),
        threads: fields.get(19)?.to_string(),
    })
}

fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
}
