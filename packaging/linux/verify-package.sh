#!/usr/bin/env bash
set -euo pipefail

kind="$1"
dir="$(cd "$2" && pwd)"

smoke() {
  Xvfb :99 -screen 0 1600x1000x24 >/dev/null 2>&1 &
  local xvfb=$!
  sleep 2
  local out
  out="$(mktemp -d)"
  DISPLAY=:99 timeout 300 "$@" --smoke "$out" || true
  kill "$xvfb" || true
  cat "$out/report.txt"
  grep -q '^PASS' "$out/report.txt"
}

case "$kind" in
  arch)
    pacman -Syu --noconfirm --needed base-devel namcap xorg-server-xvfb mesa >/dev/null
    useradd -m builder
    mkdir -p /home/builder/build
    cp "$dir"/aur/nativedraft-bin/PKGBUILD "$dir"/nativedraft-v*-linux-x86_64.tar.gz /home/builder/build/
    chown -R builder /home/builder/build
    su builder -c "cd ~/build && makepkg -f --noconfirm && makepkg --printsrcinfo > .SRCINFO"
    namcap /home/builder/build/PKGBUILD
    namcap /home/builder/build/nativedraft-bin-*.pkg.tar.zst
    pacman -U --noconfirm /home/builder/build/nativedraft-bin-*.pkg.tar.zst
    test -x /usr/bin/nativedraft
    test -f /usr/share/applications/nativedraft.desktop
    nativedraft --version
    smoke nativedraft
    ;;
  rpm)
    dnf install -y "$dir"/nativedraft-*.x86_64.rpm xorg-x11-server-Xvfb mesa-dri-drivers >/dev/null
    rpm -ql nativedraft
    nativedraft --version
    smoke nativedraft
    ;;
  deb)
    export DEBIAN_FRONTEND=noninteractive
    apt-get update >/dev/null
    apt-get install -y "$dir"/nativedraft_*_amd64.deb xvfb libgl1-mesa-dri >/dev/null
    dpkg -L nativedraft
    nativedraft --version
    smoke nativedraft
    ;;
  appimage)
    if command -v apt-get >/dev/null 2>&1; then
      export DEBIAN_FRONTEND=noninteractive
      apt-get update >/dev/null
      apt-get install -y xvfb libgl1 libegl1 libgl1-mesa-dri libxkbcommon0 libwayland-client0 libx11-6 libxcursor1 libxrandr2 libxi6 >/dev/null
    else
      dnf install -y xorg-x11-server-Xvfb mesa-dri-drivers mesa-libGL mesa-libEGL libxkbcommon libwayland-client libX11 libXcursor libXrandr libXi >/dev/null
    fi
    app="$(ls "$dir"/nativedraft-v*-linux-x86_64.AppImage)"
    chmod +x "$app"
    "$app" --appimage-extract-and-run --version
    smoke "$app" --appimage-extract-and-run
    ;;
  *)
    echo "usage: verify-package.sh arch|rpm|deb|appimage <dir>" >&2
    exit 2
    ;;
esac
