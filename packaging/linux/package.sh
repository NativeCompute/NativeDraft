#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 3 ]; then
  echo "usage: packaging/linux/package.sh <version> <nativedraft binary> <out dir> [release tarball]" >&2
  exit 2
fi

version="${1#v}"
bin="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
out="$(mkdir -p "$3" && cd "$3" && pwd)"
tarball="${4:-}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

NFPM_VERSION=2.41.1
fetch_tools() {
  mkdir -p "$work/tools"
  if ! command -v nfpm >/dev/null 2>&1; then
    curl -fsSL "https://github.com/goreleaser/nfpm/releases/download/v${NFPM_VERSION}/nfpm_${NFPM_VERSION}_Linux_x86_64.tar.gz" | tar -xz -C "$work/tools" nfpm
  fi
  if ! command -v linuxdeploy >/dev/null 2>&1; then
    curl -fsSL -o "$work/tools/linuxdeploy" "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
    chmod +x "$work/tools/linuxdeploy"
  fi
  export PATH="$work/tools:$PATH"
}

stage() {
  local root="$1"
  install -Dm755 "$bin" "$root/usr/bin/nativedraft"
  install -Dm644 "$here/packaging/nativedraft.desktop" "$root/usr/share/applications/nativedraft.desktop"
  install -Dm644 "$here/packaging/nativedraft.svg" "$root/usr/share/icons/hicolor/scalable/apps/nativedraft.svg"
  for px in 256 512; do
    mkdir -p "$root/usr/share/icons/hicolor/${px}x${px}/apps"
    "$bin" --emit-png "$px" "$root/usr/share/icons/hicolor/${px}x${px}/apps/nativedraft.png" >/dev/null
  done
  install -Dm644 "$here/LICENSE" "$root/usr/share/doc/nativedraft/copyright"
}

sum() {
  (cd "$(dirname "$1")" && sha256sum "$(basename "$1")" > "$(basename "$1").sha256")
}

say() { printf ':: %s\n' "$*"; }

say "Fetching nfpm and linuxdeploy"
fetch_tools
say "Staging the files"
stage "$work/stage"

sed -e "s|\${VERSION}|${version}|g" -e "s|\${STAGE}|${work}/stage|g" "$here/packaging/nfpm.yaml" > "$work/nfpm.yaml"
for kind in deb rpm; do
  say "Building the .$kind"
  nfpm package -f "$work/nfpm.yaml" -p "$kind" -t "$work/"
done
deb="$(ls "$work"/*.deb)"
rpm="$(ls "$work"/*.rpm)"
mv "$deb" "$out/nativedraft_${version}_amd64.deb"
mv "$rpm" "$out/nativedraft-${version}-1.x86_64.rpm"
sum "$out/nativedraft_${version}_amd64.deb"
sum "$out/nativedraft-${version}-1.x86_64.rpm"

say "Building the AppImage"
stage "$work/AppDir"
extra=()
for lib in libxkbcommon.so.0 libxkbcommon-x11.so.0; do
  found="$(ldconfig -p | awk -v l="$lib" '$1 == l && /x86-64/ { print $NF; exit }')"
  if [ -n "$found" ]; then
    extra+=(--library "$found")
  else
    say "Warning: $lib isn't installed here, so the AppImage won't carry it"
  fi
done
(
  cd "$work"
  ARCH=x86_64 LDAI_OUTPUT="nativedraft.AppImage" linuxdeploy --appimage-extract-and-run \
    --appdir AppDir \
    --desktop-file AppDir/usr/share/applications/nativedraft.desktop \
    --icon-file AppDir/usr/share/icons/hicolor/256x256/apps/nativedraft.png \
    "${extra[@]}" \
    --output appimage
)
mv "$work/nativedraft.AppImage" "$out/nativedraft-v${version}-linux-x86_64.AppImage"
chmod +x "$out/nativedraft-v${version}-linux-x86_64.AppImage"
sum "$out/nativedraft-v${version}-linux-x86_64.AppImage"

mkdir -p "$out/aur/nativedraft-bin" "$out/aur/nativedraft"
bin_sum="SKIP"
if [ -n "$tarball" ] && [ -f "$tarball" ]; then
  bin_sum="$(sha256sum "$tarball" | cut -d' ' -f1)"
fi
sed -e "s/^pkgver=.*/pkgver=${version}/" -e "s/^sha256sums=.*/sha256sums=('${bin_sum}')/" \
  "$here/packaging/aur/nativedraft-bin/PKGBUILD" > "$out/aur/nativedraft-bin/PKGBUILD"
sed -e "s/^pkgver=.*/pkgver=${version}/" \
  "$here/packaging/aur/nativedraft/PKGBUILD" > "$out/aur/nativedraft/PKGBUILD"

echo "packaged ${version} into ${out}"
ls -1 "$out"
