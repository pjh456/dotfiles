#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ETC="$REPO_ROOT/etc"

SERVICES=(waybar swaync hyprpaper hypridle fcitx5-daemon copyq)
UV_TOOLS=(hyprconf2lua ruff zhihu-tui)

with_systemd=1
with_sudo=1
with_uv=1

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Deploy dotfiles from etc/ into \$HOME as symlinks, enable systemd user
services, install uv tools, and set up the on-demand bluetooth sudoers
rule. Idempotent: safe to re-run.

Options:
  --no-systemd  skip daemon-reload / service enabling
  --no-sudo     skip the bluetooth sudoers rule
  --no-uv       skip uv tool installs
  -h, --help    show this help
EOF
}

for arg in "$@"; do
  case "$arg" in
    --no-systemd) with_systemd=0 ;;
    --no-sudo) with_sudo=0 ;;
    --no-uv) with_uv=0 ;;
    -h | --help) usage; exit 0 ;;
    *) echo "error: unknown option: $arg" >&2; usage; exit 1 ;;
  esac
done

[ -d "$ETC" ] || { echo "error: $ETC not found (run from a clone of this repo)" >&2; exit 1; }

# --- 1. collect conflicts, back them up, then symlink everything ----------
backup_dir="$HOME/.dotfiles-backup-$(date +%Y%m%d-%H%M%S)"
to_backup=()

while IFS= read -r -d '' f; do
  rel="${f#"$ETC"/}"
  target="$HOME/$rel"
  if [ -L "$target" ]; then
    # already our symlink -> idempotent no-op
    [ "$(readlink -f "$target")" = "$(readlink -f "$f")" ] && continue
  elif [ ! -e "$target" ]; then
    continue
  fi
  to_backup+=("$target")
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

while IFS= read -r -d '' f; do
  rel="${f#"$ETC"/}"
  mkdir -p "$HOME/$(dirname "$rel")"
  ln -sfn "$f" "$HOME/$rel"
done < <(find "$ETC" -type f -print0)
echo "symlinked $(find "$ETC" -type f | wc -l) file(s) into \$HOME"

# --- 2. systemd user services ---------------------------------------------
if [ "$with_systemd" -eq 1 ] && command -v systemctl >/dev/null 2>&1; then
  if systemctl --user daemon-reload 2>/dev/null; then
    for s in "${SERVICES[@]}"; do
      if systemctl --user enable "$s".service 2>/dev/null; then
        echo "enabled $s.service"
      else
        echo "skipped $s.service (unit missing?)"
      fi
    done
  else
    echo "warning: no systemd user bus, skipped service enabling"
  fi
fi

# --- 3. uv tools -----------------------------------------------------------
if [ "$with_uv" -eq 1 ]; then
  if command -v uv >/dev/null 2>&1; then
    for t in "${UV_TOOLS[@]}"; do
      uv tool install --quiet "$t" && echo "installed uv tool: $t"
    done
  else
    echo "note: uv not found, skipped uv tools (uv tool install ${UV_TOOLS[*]})"
  fi
fi

# --- 4. on-demand bluetooth sudoers rule ------------------------------------
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
