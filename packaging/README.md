# Packaging NativeDraft

Every release builds and checks these on GitHub Actions (`.github/workflows/release.yml`), and `.github/workflows/packages.yml` does the same whenever packaging changes.

| Package | Made by | Checked on |
|---|---|---|
| `nativedraft-<v>-linux-x86_64.tar.gz` | the Linux build | the updater and `install.sh` |
| `nativedraft_<v>_amd64.deb` | `linux/package.sh` (nfpm) | Debian 12, Ubuntu 22.04 |
| `nativedraft-<v>-1.x86_64.rpm` | `linux/package.sh` (nfpm) | Fedora 41 |
| `nativedraft-v<v>-linux-x86_64.AppImage` | `linux/package.sh` (linuxdeploy) | Ubuntu 24.04, Fedora 41 |
| AUR `nativedraft-bin` and `nativedraft` | `aur/*/PKGBUILD`, versions and checksums filled in by `linux/package.sh` | Arch Linux, with `namcap` |
| Nix flake | `../flake.nix` and `nix/package.nix` | Ubuntu, with Nix's own Mesa |

Each check installs the package in a clean container, opens NativeDraft and runs the whole `--smoke` pass, so a package that would not start never ships.

Every release also carries copies whose names never change, so the website can link straight to the newest one:

- `https://github.com/NativeCompute/NativeDraft/releases/latest/download/nativedraft-linux-x86_64.AppImage`
- `…/nativedraft-linux-x86_64.tar.gz`
- `…/nativedraft-windows-x86_64.zip`
- `…/nativedraft-macos.zip`

## Publishing

**AUR.** Create the `nativedraft-bin` and `nativedraft` packages on aur.archlinux.org once, under your account. Add that account's SSH private key as the repository secret `AUR_SSH_KEY`, and every release pushes both PKGBUILDs with a fresh `.SRCINFO`. Without the secret, the PKGBUILDs are attached to the release to push by hand:

```bash
git clone ssh://aur@aur.archlinux.org/nativedraft-bin.git
cp nativedraft-bin.PKGBUILD nativedraft-bin/PKGBUILD
cd nativedraft-bin && makepkg --printsrcinfo > .SRCINFO && git add PKGBUILD .SRCINFO && git commit -m v3.0.0 && git push
```

**Nix.** The flake works as it is: `nix run github:NativeCompute/NativeDraft`. For nixpkgs, `nix/package.nix` is written in nixpkgs style. Two changes before opening the pull request:

- point `src` at `fetchFromGitHub` for the release tag;
- swap `allowBuiltinFetchGit = true` for `outputHashes = { "ncf-0.4.0" = "sha256-…"; }`.

NCF is a git dependency, and nixpkgs needs its hash written down.

On a computer that isn't NixOS, the wrapper points at Nix's own Mesa drivers, so the window opens without `nixGL`. On NixOS it leaves the system's drivers alone.

**Debian, Fedora and openSUSE repositories.** The `.deb` and `.rpm` install with the system's package manager as they are. A PPA, COPR or OBS project can come later.

**Flatpak** is planned for 3.1.
