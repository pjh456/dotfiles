#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ETC="$REPO_ROOT/etc"

SERVICES=(
  waybar:waybar
  swaync:swaync
  hyprpaper:hyprpaper
  hypridle:hypridle
  fcitx5-daemon:fcitx5
  copyq:copyq
)

# packages that travel together: excluding the first also excludes the rest
PKG_ALIASES=(
  tlp:tlp-pd
)

with_systemd=1
with_sudo=1
with_packages=0
with_restore=0
restore_dir=
NO_PKGS=()

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Deploy dotfiles from etc/ into \$HOME as regular file copies, enable
systemd user services, and set up the on-demand bluetooth sudoers rule.
Idempotent: safe to re-run.

Options:
  --packages          also install required packages (arch, debian)
  --restore [DIR]     undo a deployment: remove deployed files and restore
                      backed-up originals from DIR (default: newest
                      ~/.dotfiles-backup-*)
  --no-systemd        skip daemon-reload / service enabling
  --no-sudo           skip the bluetooth sudoers rule
  --no-<pkg>          do not install package <pkg> (requires --packages; the
                      name must exist in the detected distro's package lists)
  -h, --help          show this help
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --packages) with_packages=1; shift ;;
    --restore)
      with_restore=1
      shift
      if [ $# -gt 0 ] && [ -d "$1" ]; then restore_dir="$1"; shift; fi
      ;;
    --no-systemd) with_systemd=0; shift ;;
    --no-sudo) with_sudo=0; shift ;;
    --no-*)
      NO_PKGS+=("${1#--no-}")
      shift
      ;;
    -h | --help) usage; exit 0 ;;
    *) echo "error: unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

if [ "${#NO_PKGS[@]}" -gt 0 ] && [ "$with_packages" -eq 0 ]; then
  echo "error: --no-<pkg> filters package installs; use it with --packages" >&2
  exit 1
fi

[ -d "$ETC" ] || { echo "error: $ETC not found (run from a clone of this repo)" >&2; exit 1; }

# --- R. restore: undo a previous deployment --------------------------------
manifest_file="$HOME/.dotfiles-deployed"

if [ "$with_restore" -eq 1 ]; then
  if [ -z "$restore_dir" ]; then
    restore_dir=$(ls -1d "$HOME"/.dotfiles-backup-* 2>/dev/null | sort | tail -1)
  fi
  [ -n "$restore_dir" ] && [ -d "$restore_dir" ] || {
    echo "error: backup dir not found: ${restore_dir:-<none>}" >&2; exit 1;
  }

  restore_one() {
    local src="$1" target
    target="$HOME/${src#"$restore_dir"/}"
    if [ -e "$target" ]; then
      rm -f "$target"
    fi
    mkdir -p "$(dirname "$target")"
    mv "$src" "$target"
    RESTORED=$((RESTORED + 1))
  }

  RESTORED=0
  if [ -f "$manifest_file" ]; then
    while IFS= read -r rel; do
      [ -n "$rel" ] || continue
      if [ -e "$restore_dir/$rel" ]; then
        restore_one "$restore_dir/$rel"
      elif [ -e "$HOME/$rel" ]; then
        rm -f "$HOME/$rel"
      fi
    done < "$manifest_file"
  else
    echo "warning: no manifest ($manifest_file), restoring everything in the backup" >&2
    while IFS= read -r -d '' b; do
      restore_one "$b"
    done < <(find "$restore_dir" -type f -print0)
  fi

  rm -f "$manifest_file"
  echo "restored $RESTORED file(s) from $restore_dir (deployed files removed)"
  echo "note: systemd enabling and the sudoers rule are not rolled back"
  exit 0
fi

# --- 0. optional: install required packages --------------------------------
if [ "$with_packages" -eq 1 ]; then
  . /etc/os-release

  SEEN_PKGS=()

  aliased_out() {
    local pair main extra n
    for pair in "${PKG_ALIASES[@]}"; do
      main="${pair%%:*}"
      extra="${pair##*:}"
      [ "$extra" = "$1" ] || continue
      for n in "${NO_PKGS[@]}"; do
        [ "$n" = "$main" ] && return 0
      done
    done
    return 1
  }

  load_list() {
    local file="$REPO_ROOT/$1"
    local -n _pkgs="$2"
    local p n skip
    [ -f "$file" ] || { echo "error: missing $file" >&2; exit 1; }
    _pkgs=()
    while IFS= read -r p; do
      [ -n "$p" ] || continue
      SEEN_PKGS+=("$p")
      skip=0
      for n in "${NO_PKGS[@]}"; do
        if [ "$p" = "$n" ]; then
          skip=1
          break
        fi
      done
      if [ "$skip" -eq 0 ] && aliased_out "$p"; then
        skip=1
      fi
      if [ "$skip" -eq 0 ]; then
        _pkgs+=("$p")
      fi
    done < <(grep -vE '^[[:space:]]*(#|$)' "$file")
  }

  check_no_pkgs() {
    local n p hit
    for n in "${NO_PKGS[@]}"; do
      hit=0
      for p in "${SEEN_PKGS[@]}"; do
        if [ "$p" = "$n" ]; then
          hit=1
          break
        fi
      done
      if [ "$hit" -eq 0 ]; then
        echo "error: --no-$n: '$n' is not in the $ID package lists" >&2
        echo "available: ${SEEN_PKGS[*]}" >&2
        exit 1
      fi
      case "$n" in
        tlp)
          echo "note: without tlp the power-mode module shows N/A and the mpv profile stays 'balanced'"
          ;;
        lm_sensors)
          echo "note: without lm_sensors the waybar temperature module shows N/A"
          ;;
        pass)
          echo "note: without pass, SNYK_TOKEN/HF_TOKEN stay unset and the hyprlock password prompt has no source"
          ;;
      esac
    done
  }

  SUDO=
  [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1 && SUDO=sudo

  case "$ID" in
    arch)
      load_list packages/arch-official.txt PKGS_OFFICIAL
      load_list packages/arch-aur.txt PKGS_AUR
      check_no_pkgs

      echo "installing official packages (pacman)..."
      $SUDO pacman -S --needed --noconfirm "${PKGS_OFFICIAL[@]}"

      helper=
      for h in yay paru buttercup; do
        command -v "$h" >/dev/null 2>&1 && helper=$h && break
      done
      if [ -n "$helper" ]; then
        echo "installing AUR packages ($helper)..."
        $SUDO "$helper" -S --needed --noconfirm "${PKGS_AUR[@]}"
      else
        echo "note: no AUR helper (yay/paru/buttercup) found, skipped: ${PKGS_AUR[*]}"
        echo "      install manually: yay -S ${PKGS_AUR[*]}"
      fi
      ;;
    debian)
      load_list packages/debian.txt PKGS_OFFICIAL
      load_list packages/debian-manual.txt PKGS_MANUAL
      check_no_pkgs

      echo "installing packages (apt, Debian)..."
      DEBIAN_FRONTEND=noninteractive $SUDO apt-get update -qq
      DEBIAN_FRONTEND=noninteractive $SUDO apt-get install --yes --no-install-recommends "${PKGS_OFFICIAL[@]}"

      if [ "${#PKGS_MANUAL[@]}" -gt 0 ]; then
        echo "note: no Debian packages for: ${PKGS_MANUAL[*]}"
        echo "      build from source (see README 'Manual builds')"
      fi
      ;;
    *)
      echo "error: --packages supports 'arch' and 'debian' (detected: $ID)" >&2
      exit 1
      ;;
  esac
fi

# --- 1. back up real conflicts, then copy everything from etc/ -------------
in_manifest() {
  [ -f "$manifest_file" ] && grep -qxF "$1" "$manifest_file"
}

backup_dir="$HOME/.dotfiles-backup-$(date +%Y%m%d-%H%M%S)"
to_backup=()

# pass 1: clear symlink-era artifacts, collect real conflicts for backup
while IFS= read -r -d '' f; do
  rel="${f#"$ETC"/}"
  target="$HOME/$rel"
  if [ -L "$target" ] && [ "$(readlink -f "$target")" = "$(readlink -f "$f")" ]; then
    rm "$target"
  elif [ -e "$target" ] && { ! in_manifest "$rel" || ! cmp -s "$f" "$target"; }; then
    to_backup+=("$target")
  fi
done < <(find "$ETC" -type f -print0)

if [ "${#to_backup[@]}" -gt 0 ]; then
  mkdir -p "$backup_dir"
  for t in "${to_backup[@]}"; do
    rel="${t#"$HOME"/}"
    mkdir -p "$backup_dir/$(dirname "$rel")"
    mv "$t" "$backup_dir/$rel"
  done
  echo "backed up ${#to_backup[@]} existing file(s) -> $backup_dir"
fi

# pass 2: copy (skip files already up to date from a previous deployment)
while IFS= read -r -d '' f; do
  rel="${f#"$ETC"/}"
  if [ -e "$HOME/$rel" ] && in_manifest "$rel" && cmp -s "$f" "$HOME/$rel"; then
    continue
  fi
  mkdir -p "$HOME/$(dirname "$rel")"
  cp -a "$f" "$HOME/$rel"
done < <(find "$ETC" -type f -print0)

find "$ETC" -type f | sed "s|$ETC/||" > "$manifest_file"
echo "deployed $(wc -l < "$manifest_file") file(s) into \$HOME (copies; manifest: $manifest_file)"

# --- 2. systemd user services ---------------------------------------------
if [ "$with_systemd" -eq 1 ] && command -v systemctl >/dev/null 2>&1; then
  if systemctl --user daemon-reload 2>/dev/null; then
    for s in "${SERVICES[@]}"; do
      unit="${s%%:*}"
      bin="${s##*:}"
      if ! command -v "$bin" >/dev/null 2>&1; then
        echo "skipped $unit.service ($bin not installed)"
        continue
      fi
      if systemctl --user enable "$unit".service 2>/dev/null; then
        echo "enabled $unit.service"
      else
        echo "skipped $unit.service (unit missing?)"
      fi
    done
  else
    echo "warning: no systemd user bus, skipped service enabling"
  fi
fi

# --- 3. on-demand bluetooth sudoers rule ------------------------------------
if [ "$with_sudo" -eq 1 ]; then
  rule="/etc/sudoers.d/bluetooth-ondemand"
  if [ -e "$rule" ]; then
    echo "sudoers rule present: $rule"
  elif command -v sudo >/dev/null 2>&1; then
    line="$USER ALL=(ALL) NOPASSWD: /usr/bin/systemctl start bluetooth.service, /usr/bin/systemctl stop bluetooth.service"
    if echo "$line" | sudo tee "$rule" >/dev/null; then
      sudo chmod 440 "$rule"
      sudo visudo -c -f "$rule" >/dev/null
      echo "installed sudoers rule: $rule"
    else
      echo "warning: could not install sudoers rule (sudo failed?)"
    fi
  else
    echo "note: sudo not available, skipped sudoers rule"
  fi
fi

echo "done. open a new shell (or: source ~/.bashrc)"
