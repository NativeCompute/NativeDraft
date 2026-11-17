{
  lib,
  rustPlatform,
  src,
  makeWrapper,
  pkg-config,
  libGL,
  mesa,
  libxkbcommon,
  wayland,
  xorg,
}:

let
  runtimeLibs = [
    libGL
    libxkbcommon
    wayland
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
  ];
in
rustPlatform.buildRustPackage {
  pname = "nativedraft";
  version = "3.0.0";

  inherit src;

  cargoLock = {
    lockFile = src + "/Cargo.lock";
    allowBuiltinFetchGit = true;
  };

  nativeBuildInputs = [
    makeWrapper
    pkg-config
  ];

  buildInputs = runtimeLibs;

  doCheck = false;

  postInstall = ''
    install -Dm644 packaging/nativedraft.desktop $out/share/applications/nativedraft.desktop
    install -Dm644 packaging/nativedraft.svg $out/share/icons/hicolor/scalable/apps/nativedraft.svg
    wrapProgram $out/bin/nativedraft \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs} \
      --run 'if [ ! -e /run/opengl-driver ]; then export __EGL_VENDOR_LIBRARY_DIRS="''${__EGL_VENDOR_LIBRARY_DIRS:-${mesa}/share/glvnd/egl_vendor.d}"; export LIBGL_DRIVERS_PATH="''${LIBGL_DRIVERS_PATH:-${mesa}/lib/dri}"; export __GLX_VENDOR_LIBRARY_NAME="''${__GLX_VENDOR_LIBRARY_NAME:-mesa}"; export LD_LIBRARY_PATH="${mesa}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"; fi'
  '';

  meta = {
    description = "Free screenwriting app with industry format as you type, branching timelines, and PDF, Final Draft and Fountain export";
    homepage = "https://nativecompute.net/nativedraft";
    changelog = "https://github.com/NativeCompute/NativeDraft/releases";
    license = lib.licenses.mit;
    mainProgram = "nativedraft";
    platforms = lib.platforms.linux;
  };
}
