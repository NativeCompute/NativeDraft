#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

if [ "$(uname -s)" = "Darwin" ]; then
  if [ ! -f "$here/install-macos.sh" ]; then
    echo "This copy of NativeDraft is for Linux. On a Mac, install it with:" >&2
    echo "    curl -fsSL https://github.com/NativeCompute/NativeDraft/releases/latest/download/install-macos.sh | bash" >&2
    exit 1
  fi
  if [ -f "$here/Cargo.toml" ]; then
    exec /bin/bash "$here/install-macos.sh" --from-source ${1+"$@"}
  fi
  exec /bin/bash "$here/install-macos.sh" ${1+"$@"}
fi

say() { printf '\033[1;31m::\033[0m %s\n' "$*"; }

BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/scalable/apps"
PNG_DIR="$HOME/.local/share/icons/hicolor"

refresh_desktop() {
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" || true
  command -v gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
  command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 >/dev/null 2>&1 || true
  command -v kbuildsycoca5 >/dev/null 2>&1 && kbuildsycoca5 >/dev/null 2>&1 || true
}

SHARE="${XDG_DATA_HOME:-$HOME/.local/share}"
DATA_DIR="$SHARE/nativedraft"
if [ ! -e "$DATA_DIR" ] && [ -d "$SHARE/blackline" ]; then
  DATA_DIR="$SHARE/blackline"
elif [ ! -e "$DATA_DIR" ] && [ -d "$SHARE/northstar" ]; then
  DATA_DIR="$SHARE/northstar"
fi

remove_northstar() {
  rm -f "$BIN_DIR/northstar" "$APP_DIR/northstar.desktop" "$ICON_DIR/northstar.svg"
}

remove_blackline_launcher() {
  rm -f "$APP_DIR/blackline.desktop" "$ICON_DIR/blackline.svg" "$PNG_DIR/256x256/apps/blackline.png" "$PNG_DIR/512x512/apps/blackline.png"
}

if [ "${1:-}" = "--uninstall" ]; then
  rm -f "$BIN_DIR/nativedraft" "$APP_DIR/nativedraft.desktop" "$ICON_DIR/nativedraft.svg" "$PNG_DIR/256x256/apps/nativedraft.png" "$PNG_DIR/512x512/apps/nativedraft.png"
  rm -f "$BIN_DIR/blackline"
  remove_blackline_launcher
  remove_northstar
  refresh_desktop
  say "NativeDraft removed. Your scripts are still in $DATA_DIR"
  exit 0
fi

PREBUILT=0
if [ -x "$here/nativedraft" ] && [ ! -f "$here/Cargo.toml" ]; then
  PREBUILT=1
  BUILT="$here/nativedraft"
elif [ -x "$here/blackline" ] && [ ! -f "$here/Cargo.toml" ]; then
  PREBUILT=1
  BUILT="$here/blackline"
else
  BUILT="target/release/nativedraft"
fi

if [ "$PREBUILT" = 0 ]; then
if command -v apt-get >/dev/null 2>&1 && command -v dpkg >/dev/null 2>&1; then
  PKGS=(
    build-essential pkg-config
    libgl1-mesa-dev
    libx11-dev libxcursor-dev libxrandr-dev libxi-dev
    libxkbcommon-dev libxkbcommon-x11-dev
    libwayland-dev
  )
  missing=()
  for p in "${PKGS[@]}"; do
    dpkg -s "$p" >/dev/null 2>&1 || missing+=("$p")
  done
  if [ ${#missing[@]} -gt 0 ]; then
    say "Installing build dependencies: ${missing[*]}"
    sudo apt-get update || say "apt-get update reported errors (often a stale PPA); carrying on"
    sudo apt-get install -y "${missing[@]}"
  fi
elif command -v pacman >/dev/null 2>&1; then
  say "Installing build dependencies with pacman"
  sudo pacman -S --needed --noconfirm base-devel pkgconf libglvnd libx11 libxcursor libxrandr libxi libxkbcommon libxkbcommon-x11 wayland
elif command -v dnf >/dev/null 2>&1; then
  say "Installing build dependencies with dnf"
  sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config mesa-libGL-devel libX11-devel libXcursor-devel libXrandr-devel libXi-devel libxkbcommon-devel libxkbcommon-x11-devel wayland-devel
elif command -v zypper >/dev/null 2>&1; then
  say "Installing build dependencies with zypper"
  sudo zypper --non-interactive install gcc gcc-c++ make pkg-config Mesa-libGL-devel libX11-devel libXcursor-devel libXrandr-devel libXi-devel libxkbcommon-devel libxkbcommon-x11-devel wayland-devel
else
  say "Could not tell which package manager this is. Install a C compiler, pkg-config and the OpenGL, X11, xkbcommon and Wayland development files, then run this again if the build fails."
fi

if ! command -v cargo >/dev/null 2>&1 && [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  cat <<'EOF'
Rust was not found. Install it with rustup (recommended over apt, which ships
an older toolchain):

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source "$HOME/.cargo/env"

Then run this script again.
EOF
  exit 1
fi

say "Building (this takes a few minutes the first time)"
cargo build --release
fi

if [ -d "$DATA_DIR" ]; then
  BACKUP="${DATA_DIR}-backup-$(date +%Y%m%d-%H%M%S)"
  cp -a "$DATA_DIR" "$BACKUP"
  say "Your library was copied to $BACKUP first"
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"
remove_northstar
install -m 755 "$BUILT" "$BIN_DIR/nativedraft.new"
mv -f "$BIN_DIR/nativedraft.new" "$BIN_DIR/nativedraft"
install -m 644 packaging/nativedraft.desktop "$APP_DIR/nativedraft.desktop"

if [ -e "$BIN_DIR/blackline" ] || [ -L "$BIN_DIR/blackline" ] || [ -e "$APP_DIR/blackline.desktop" ]; then
  say "Blackline is now NativeDraft; replacing the old launcher (your scripts stay where they are)"
  remove_blackline_launcher
  ln -sfn nativedraft "$BIN_DIR/blackline"
fi

say "Drawing the icon"
"$BIN_DIR/nativedraft" --emit-icon "$ICON_DIR/nativedraft.svg" >/dev/null
for px in 256 512; do
  mkdir -p "$PNG_DIR/${px}x${px}/apps"
  "$BIN_DIR/nativedraft" --emit-png "$px" "$PNG_DIR/${px}x${px}/apps/nativedraft.png" >/dev/null
done
install -m 644 "$ICON_DIR/nativedraft.svg" packaging/nativedraft.svg

refresh_desktop

say "Installed to $BIN_DIR/nativedraft"
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BIN_DIR"; then
  say "Note: $BIN_DIR is not on your PATH. Add this to ~/.bashrc or ~/.zshrc:"
  echo '    export PATH="$HOME/.local/bin:$PATH"'
fi
say "Scripts live in $DATA_DIR/scripts"
say "Launch it from the application menu, or run: nativedraft"
