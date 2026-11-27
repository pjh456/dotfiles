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
  power-profile:dotctl
  hyprland-guard:dotctl
  mpv-profile-watch:dotctl
)

# packages that travel together: excluding the first also excludes the rest
PKG_ALIASES=(
  tlp:tlp-pd
)

with_systemd=1
with_sudo=1
with_packages=0
with_restore=0
with_cli=1
cli_from_source=0
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
  --with-cli          install/update the dotctl Rust CLI (default; downloads
                      the pinned release, falls back to a local cargo build)
  --cli-from-source   build dotctl locally instead of downloading the release
  --no-cli            do not install/update dotctl
  --no-systemd        skip daemon-reload / service enabling
  --no-sudo           skip the system-level tweaks (bluetooth sudoers rule,
                      acpid media-key patch)
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
    --with-cli) with_cli=1; shift ;;
    --cli-from-source) with_cli=1; cli_from_source=1; shift ;;
    --no-cli) with_cli=0; shift ;;
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
    restore_dir=$(ls -1d "$HOME"/.dotfiles-backup-* 2>/dev/null | sort | tail -1 || true)
  fi
  clean_restore=0
  if [ -z "$restore_dir" ] || [ ! -d "$restore_dir" ]; then
    if [ -f "$manifest_file" ]; then
      echo "no backup dir (clean install); removing deployed files only"
      restore_dir="/nonexistent"
      clean_restore=1
    else
      echo "error: backup dir not found: ${restore_dir:-<none>}" >&2
      exit 1
    fi
  fi

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

  if [ -f "$HOME/.dotfiles-cli-version" ]; then
    rm -f "$HOME/.local/bin/dotctl" "$HOME/.dotfiles-cli-version"
    echo "removed the dotctl binary installed by this repo"
  fi

  rm -f "$manifest_file"
  if [ "$clean_restore" -eq 1 ]; then
    echo "removed deployed files (there was no backup to restore)"
  else
    echo "restored $RESTORED file(s) from $restore_dir (deployed files removed)"
  fi
  echo "note: systemd enabling, the sudoers rule and the acpid media-key patch are not rolled back"
  exit 0
fi

# --- 0b. optional: install the dotctl CLI ----------------------------------
install_dotctl() {
  local dest="$HOME/.local/bin/dotctl"
  local version tag asset base tmp
  version=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$REPO_ROOT/dotctl/Cargo.toml" | head -1)
  [ -n "$version" ] || { echo "error: cannot read dotctl version from dotctl/Cargo.toml" >&2; return 1; }
  tag="${DOTCTL_TAG:-dotctl-v$version}"
  asset="dotctl-x86_64-unknown-linux-musl.tar.gz"
  base="${DOTCTL_BASE_URL:-https://github.com/pjh456/dotfiles/releases/download/$tag}"

  mkdir -p "$HOME/.local/bin"

  build_from_source() {
    command -v cargo >/dev/null 2>&1 || {
      echo "error: dotctl release unavailable and cargo is not installed" >&2
      echo "       install Rust, or set DOTCTL_BASE_URL to a mirror" >&2
      return 1
    }
    echo "building dotctl from source..."
    if ! cargo build --locked --release --manifest-path "$REPO_ROOT/dotctl/Cargo.toml"; then
      echo "error: cargo build failed" >&2
      return 1
    fi
    install -m0755 "$REPO_ROOT/dotctl/target/release/dotctl" "$dest.new"
  }

  if [ "$cli_from_source" -eq 1 ] || ! command -v curl >/dev/null 2>&1; then
    build_from_source || return 1
  elif [ "$(uname -m)" != "x86_64" ]; then
    echo "note: no prebuilt dotctl for $(uname -m); building from source"
    build_from_source || return 1
  else
    tmp=$(mktemp -d)
    if curl -fsSL "$base/$asset" -o "$tmp/$asset" 2>/dev/null &&
      curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" 2>/dev/null; then
      if ! (cd "$tmp" && grep -F "  $asset" SHA256SUMS | sha256sum -c - >/dev/null); then
        rm -rf "$tmp"
        echo "error: dotctl checksum verification failed" >&2
        return 1
      fi
      tar -xzf "$tmp/$asset" -C "$tmp"
      install -m0755 "$tmp/dotctl" "$dest.new"
      rm -rf "$tmp"
    else
      rm -rf "$tmp"
      echo "note: dotctl release $tag not available; building from source"
      build_from_source || return 1
    fi
  fi

  mv "$dest.new" "$dest"
  printf '%s\n' "$tag" > "$HOME/.dotfiles-cli-version"
  echo "installed $dest ($("$dest" --version 2>/dev/null || echo unknown))"
}

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
          echo "note: without tlp the power-profile service cannot set a profile and the mpv profile stays 'balanced'"
          ;;
        lm_sensors)
          echo "note: without lm_sensors the waybar temperature module shows N/A"
          ;;
        pass)
          echo "note: without pass, SNYK_TOKEN/HF_TOKEN stay unset and the hyprlock password prompt has no source"
          ;;
        blueman)
          echo "note: without blueman, clicking the waybar bluetooth icon toggles the service but no manager GUI opens"
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

new_manifest="$manifest_file.new"
find "$ETC" -type f | sed "s|$ETC/||" > "$new_manifest"

# Files deployed by an older revision but removed from etc/ are moved to the
# backup dir instead of lingering forever in $HOME.
if [ -f "$manifest_file" ]; then
  while IFS= read -r rel; do
    [ -n "$rel" ] || continue
    if grep -qxF "$rel" "$new_manifest"; then
      continue
    fi
    target="$HOME/$rel"
    if [ ! -e "$target" ] && [ ! -L "$target" ]; then
      continue
    fi
    mkdir -p "$backup_dir/pruned/$(dirname "$rel")"
    mv "$target" "$backup_dir/pruned/$rel"
    echo "pruned $rel (no longer in etc/)"
  done < "$manifest_file"
fi
mv "$new_manifest" "$manifest_file"
echo "deployed $(wc -l < "$manifest_file") file(s) into \$HOME (copies; manifest: $manifest_file)"

if [ "$with_cli" -eq 1 ]; then
  install_dotctl
fi

# --- 2. systemd user services ---------------------------------------------
if [ "$with_systemd" -eq 1 ] && command -v systemctl >/dev/null 2>&1; then
  if systemctl --user daemon-reload 2>/dev/null; then
    for s in "${SERVICES[@]}"; do
      unit="${s%%:*}"
      bin="${s##*:}"
      if ! command -v "$bin" >/dev/null 2>&1 && [ ! -x "$HOME/.local/bin/$bin" ]; then
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
# /etc/sudoers.d is root-only, so an unprivileged [ -e ] never sees the rule
# and the old check re-installed it (asking for a password) on every run.
# Ask sudo with -n instead: it answers only while credentials are cached.
# Without cached credentials, trust the marker dropped after a successful
# install, so a plain re-run stays password-free.
if [ "$with_sudo" -eq 1 ]; then
  rule="/etc/sudoers.d/bluetooth-ondemand"
  marker="$HOME/.dotfiles-sudoers-bluetooth"
  rule_state=unknown
  if sudo -n true 2>/dev/null; then
    if sudo -n test -e "$rule"; then
      rule_state=present
    else
      rule_state=missing
    fi
  elif [ -e "$marker" ]; then
    rule_state=assumed
  fi

  case "$rule_state" in
    present)
      printf 'verified %s\n' "$(date -Is)" >"$marker"
      echo "sudoers rule present: $rule"
      ;;
    assumed)
      echo "sudoers rule assumed present: $rule (run 'sudo -v' first to verify)"
      ;;
    *)
      if command -v sudo >/dev/null 2>&1; then
        line="$USER ALL=(ALL) NOPASSWD: /usr/bin/systemctl start bluetooth.service, /usr/bin/systemctl stop bluetooth.service"
        if echo "$line" | sudo tee "$rule" >/dev/null; then
          sudo chmod 440 "$rule"
          sudo visudo -c -f "$rule" >/dev/null
          printf 'installed %s\n' "$(date -Is)" >"$marker"
          echo "installed sudoers rule: $rule"
        else
          echo "warning: could not install sudoers rule (sudo failed?)" >&2
        fi
      else
        echo "note: sudo not available, skipped sudoers rule"
      fi
      ;;
  esac
fi

# --- 4. keep acpid off the media keys ---------------------------------------
# Hyprland (keys.lua -> dotctl osd) owns volume/brightness. acpid's input layer
# still turns the same keycodes into button/mute, button/volumeup, ... and
# runs wpctl/brightnessctl a second time, so mute cancels itself out and
# volume/brightness step twice. Comment the command lines out and keep the
# logger lines, so `journalctl -g 'ACPI:'` still shows what arrives.
acpi_handler="${ACPI_HANDLER:-/etc/acpi/handler.sh}"
if [ "$with_sudo" -eq 1 ] && [ -f "$acpi_handler" ]; then
  acpi_patched="$(mktemp)"
  awk '
    /^[[:space:]]*#/ { print; next }
    {
      cmd = $0
      sub(/^[[:space:]]*/, "", cmd)
      if (cmd == "/usr/local/bin/volume-mute" ||
          cmd ~ /^\/usr\/local\/bin\/volume-control 1%[+-]$/ ||
          cmd ~ /^\/usr\/bin\/brightnessctl set [+]?1%-?$/) {
        match($0, /^[[:space:]]*/)
        printf "%s# %s   # disabled: Hyprland keys.lua/dotctl osd owns this key\n", substr($0, 1, RLENGTH), cmd
        next
      }
      print
    }
  ' "$acpi_handler" >"$acpi_patched"

  if cmp -s "$acpi_handler" "$acpi_patched"; then
    echo "acpid already leaves the media keys alone: $acpi_handler"
  elif ! bash -n "$acpi_patched"; then
    echo "warning: patched $acpi_handler would not parse; left untouched" >&2
  else
    acpi_sudo=()
    [ "$(id -u)" -ne 0 ] && [ ! -w "$acpi_handler" ] && acpi_sudo=(sudo)
    acpi_backup="$acpi_handler.bak-$(date +%Y%m%d-%H%M%S)"
    if "${acpi_sudo[@]}" cp -a "$acpi_handler" "$acpi_backup" &&
      "${acpi_sudo[@]}" cp "$acpi_patched" "$acpi_handler"; then
      echo "acpid no longer acts on the media keys (backup: $acpi_backup)"
    else
      echo "warning: could not patch $acpi_handler (media keys may act twice)" >&2
    fi
  fi
  rm -f "$acpi_patched"
fi

echo "done. open a new shell (or: source ~/.bashrc)"
