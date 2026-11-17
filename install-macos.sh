#!/bin/bash
if [ -z "${BASH_VERSION:-}" ]; then
  echo "Run this with bash: curl -fsSL https://github.com/NativeCompute/NativeDraft/releases/latest/download/install-macos.sh | bash" >&2
  exit 1
fi
set -euo pipefail

NATIVEDRAFT_SOURCE="${NATIVEDRAFT_SOURCE:-${BLACKLINE_SOURCE:-${NCF_INSTALL_SOURCE:-}}}"
NATIVEDRAFT_APP_DIR="${NATIVEDRAFT_APP_DIR:-${BLACKLINE_APP_DIR:-${NCF_INSTALL_APP_DIR:-}}}"
NATIVEDRAFT_NO_OPEN="${NATIVEDRAFT_NO_OPEN:-${BLACKLINE_NO_OPEN:-${NCF_INSTALL_NO_OPEN:-}}}"
NATIVEDRAFT_WAIT_SECONDS="${NATIVEDRAFT_WAIT_SECONDS:-${BLACKLINE_WAIT_SECONDS:-${NCF_INSTALL_WAIT:-}}}"

SOURCE="${NATIVEDRAFT_SOURCE:-https://github.com/NativeCompute/NativeDraft/releases/latest/download}"
ZIP="nativedraft-macos.zip"
OLD_ZIP="blackline-macos.zip"
APP_NAME="NativeDraft.app"
OLD_APP="Blackline.app"
REPLACED_OLD=0
LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
WAIT_SECONDS="${NATIVEDRAFT_WAIT_SECONDS:-600}"
WORK=""
SUPPORT="$HOME/Library/Application Support"

say() { printf '\033[1;34m::\033[0m %s\n' "$*" >&2; }
warn() { printf '\033[1;33m::\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31m::\033[0m %s\n' "$*" >&2; exit 1; }

library_dir() {
  if [ ! -e "$SUPPORT/NativeDraft" ] && [ -d "$SUPPORT/Blackline" ]; then
    echo "$SUPPORT/Blackline"
  elif [ ! -e "$SUPPORT/NativeDraft" ] && [ -d "$SUPPORT/northstar" ]; then
    echo "$SUPPORT/northstar"
  else
    echo "$SUPPORT/NativeDraft"
  fi
}

app_dir() {
  if [ -n "${NATIVEDRAFT_APP_DIR:-}" ]; then
    echo "$NATIVEDRAFT_APP_DIR"
  elif { [ -e "$HOME/Applications/$APP_NAME" ] || [ -e "$HOME/Applications/$OLD_APP" ]; } && [ ! -e "/Applications/$APP_NAME" ] && [ ! -e "/Applications/$OLD_APP" ]; then
    echo "$HOME/Applications"
  elif [ -d /Applications ] && [ -w /Applications ] && { [ ! -e "/Applications/$APP_NAME" ] || [ -w "/Applications/$APP_NAME" ]; }; then
    echo "/Applications"
  else
    echo "$HOME/Applications"
  fi
}

check_mac() {
  [ "$(uname -s)" = "Darwin" ] || die "This installer is for macOS. On Linux, use install.sh."
  local version major
  version="$(sw_vers -productVersion 2>/dev/null || echo 0)"
  major="${version%%.*}"
  case "$major" in
    ''|*[!0-9]*) major=0 ;;
  esac
  if [ "$major" -lt 11 ]; then
    die "NativeDraft needs macOS 11 Big Sur or newer. This Mac runs macOS $version."
  fi
  case "$(uname -m)" in
    arm64|x86_64) ;;
    *) die "NativeDraft runs on Apple Silicon and Intel Macs. This Mac reports $(uname -m)." ;;
  esac
}

is_running() {
  pgrep -x nativedraft >/dev/null 2>&1 || pgrep -x blackline >/dev/null 2>&1
}

wait_for_quit() {
  is_running || return 0
  say "NativeDraft is open. Quit it (Cmd+Q) and the install carries on by itself."
  local waited=0
  while is_running; do
    if [ "$waited" -ge "$WAIT_SECONDS" ]; then
      die "NativeDraft is still open, so nothing was changed. Quit it and run this again."
    fi
    sleep 2
    waited=$((waited + 2))
  done
  say "NativeDraft has closed"
}

back_up_library() {
  local data backup
  data="$(library_dir)"
  [ -d "$data" ] || return 0
  backup="${data}-backup-$(date +%Y%m%d-%H%M%S)"
  ditto "$data" "$backup" || die "Could not copy your library to $backup, so nothing was changed."
  say "Your library was copied to $backup first"
}

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    openssl dgst -sha256 "$1" | awk '{print $NF}'
  fi
}

fetch() {
  local url="$1" out="$2"
  if [ -t 2 ]; then
    curl -fL --proto '=https,file' --proto-redir '=https' --retry 3 --progress-bar -o "$out" "$url"
  else
    curl -fsSL --proto '=https,file' --proto-redir '=https' --retry 3 -o "$out" "$url"
  fi
}

download() {
  local work="$1" want got
  say "Downloading NativeDraft"
  if ! fetch "$SOURCE/$ZIP" "$work/$ZIP" 2>/dev/null; then
    fetch "$SOURCE/$OLD_ZIP" "$work/$ZIP" || die "Could not download $SOURCE/$ZIP. Check your connection and try again."
    ZIP_SUM="$OLD_ZIP.sha256"
  fi
  fetch "$SOURCE/${ZIP_SUM:-$ZIP.sha256}" "$work/$ZIP.sha256" || die "Could not download the checksum for $ZIP."
  want="$(awk '{print $1; exit}' "$work/$ZIP.sha256" | tr 'A-F' 'a-f')"
  got="$(sha256 "$work/$ZIP")"
  [ -n "$want" ] && [ "$want" = "$got" ] || die "The download is damaged (its checksum does not match). Nothing was changed. Try again."
  say "Checksum matches"
  mkdir -p "$work/unpacked"
  ditto -x -k "$work/$ZIP" "$work/unpacked" || die "Could not unpack $ZIP."
  [ -d "$work/unpacked/$APP_NAME" ] || die "The download does not contain $APP_NAME."
  echo "$work/unpacked/$APP_NAME"
}

build_from_source() {
  local work="$1" here
  here="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
  [ -f "$here/Cargo.toml" ] && [ -x "$here/packaging/macos/bundle.sh" ] || die "--from-source needs a copy of the NativeDraft source. Clone it with: git clone https://github.com/NativeCompute/NativeDraft"
  if ! xcode-select -p >/dev/null 2>&1; then
    die "Building needs Apple's command line tools. Run: xcode-select --install   then run this again."
  fi
  if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
  fi
  command -v cargo >/dev/null 2>&1 || die "Building needs Rust. Install it with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   then open a new Terminal window and run this again."
  say "Building NativeDraft (this takes a few minutes the first time)"
  "$here/packaging/macos/bundle.sh" --native --out "$work/dist" >&2 || die "The build failed. Nothing was installed."
  echo "$work/dist/$APP_NAME"
}

check_app() {
  local app="$1" version
  xattr -dr com.apple.quarantine "$app" 2>/dev/null || true
  codesign --verify --deep --strict "$app" 2>/dev/null || die "The app's signature does not check out, so it was not installed."
  [ -x "$app/Contents/MacOS/nativedraft" ] || die "The app is missing its program file."
  version="$("$app/Contents/MacOS/nativedraft" --version 2>/dev/null)" || die "The new app does not run on this Mac ($(uname -m), macOS $(sw_vers -productVersion)). Nothing was changed."
  echo "$version"
}

place_app() {
  local app="$1" dest="$2" stage old
  mkdir -p "$dest" || die "Could not create $dest."
  [ -w "$dest" ] || die "Cannot write to $dest."
  stage="$dest/.NativeDraft.app.new.$$"
  old="$dest/.NativeDraft.app.old.$$"
  rm -rf "$stage" "$old"
  ditto "$app" "$stage" || { rm -rf "$stage"; die "Could not copy the app into $dest."; }
  if [ -e "$dest/$APP_NAME" ]; then
    mv "$dest/$APP_NAME" "$old" || { rm -rf "$stage"; die "Could not move the old app aside. Nothing was changed."; }
  fi
  if ! mv "$stage" "$dest/$APP_NAME"; then
    [ -e "$old" ] && mv "$old" "$dest/$APP_NAME"
    rm -rf "$stage"
    die "Could not put the new app in place. The old one is back."
  fi
  rm -rf "$old" 2>/dev/null || warn "Could not delete the old copy at $old; it is safe to remove it by hand."
  xattr -dr com.apple.quarantine "$dest/$APP_NAME" 2>/dev/null || true
  touch "$dest/$APP_NAME"
  if [ -x "$LSREGISTER" ]; then
    "$LSREGISTER" -f "$dest/$APP_NAME" >/dev/null 2>&1 || true
  fi
  remove_old_app "$dest"
}

remove_old_app() {
  local dir
  for dir in "$1" "/Applications" "$HOME/Applications"; do
    if [ -e "$dir/$OLD_APP" ] && [ -w "$dir" ]; then
      if [ -x "$LSREGISTER" ]; then
        "$LSREGISTER" -u "$dir/$OLD_APP" >/dev/null 2>&1 || true
      fi
      if rm -rf "${dir:?}/$OLD_APP"; then
        say "Blackline is now NativeDraft; removed $dir/$OLD_APP (your scripts stay where they are)"
        REPLACED_OLD=1
      fi
    fi
  done
}

uninstall() {
  wait_for_quit
  local removed=0 dir
  local name
  for dir in "${NATIVEDRAFT_APP_DIR:-/Applications}" "$HOME/Applications"; do
    for name in "$APP_NAME" "$OLD_APP"; do
      if [ -e "$dir/$name" ]; then
        if [ -x "$LSREGISTER" ]; then
          "$LSREGISTER" -u "$dir/$name" >/dev/null 2>&1 || true
        fi
        rm -rf "${dir:?}/$name" || die "Could not remove $dir/$name."
        say "Removed $dir/$name"
        removed=1
      fi
    done
  done
  [ "$removed" = 1 ] || say "NativeDraft was not installed in /Applications or ~/Applications"
  say "Your scripts are still in $(library_dir)"
}

main() {
  local mode="download" arg
  for arg in ${1+"$@"}; do
    case "$arg" in
      --uninstall) mode="uninstall" ;;
      --from-source) [ "$mode" = "uninstall" ] || mode="source" ;;
      -h|--help)
        echo "Usage: install-macos.sh [--from-source | --uninstall]"
        exit 0
        ;;
      *) die "Unknown option: $arg" ;;
    esac
  done

  check_mac

  if [ "$mode" = "uninstall" ]; then
    uninstall
    exit 0
  fi

  local app version dest
  WORK="$(mktemp -d "${TMPDIR:-/tmp}/nativedraft-install.XXXXXX")"
  trap 'rm -rf "$WORK"' EXIT

  if [ "$mode" = "source" ]; then
    app="$(build_from_source "$WORK")"
  else
    app="$(download "$WORK")"
  fi
  version="$(check_app "$app")"
  say "Ready: $version"

  dest="$(app_dir)"
  wait_for_quit
  back_up_library
  place_app "$app" "$dest"

  say "Installed to $dest/$APP_NAME"
  if [ "$dest" != "/Applications" ] && [ -e "/Applications/$APP_NAME" ]; then
    warn "An older copy is still in /Applications and could not be replaced from this account."
  fi
  say "Scripts live in $(library_dir)/scripts"
  say "To remove it later: curl -fsSL https://github.com/NativeCompute/NativeDraft/releases/latest/download/install-macos.sh | bash -s -- --uninstall"

  if [ -z "${NATIVEDRAFT_NO_OPEN:-}" ] || [ "$REPLACED_OLD" = 1 ]; then
    open "$dest/$APP_NAME" || warn "Open NativeDraft from $dest."
  fi
}

main ${1+"$@"}
