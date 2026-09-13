# Builds noctalia-clock-iced and runs it under headless weston with Mesa's software rasterizer:
# iced screenshots of the states libnoctalia-ui's C++ e2e covers, a capture of what the compositor
# shows, and the Wayland trace of the chrome requests.
#
# `chrome = true` builds with the `wayland-chrome` feature: the patched crates in third_party/rust
# are applied through the workspace manifest and third_party/rust/Cargo.patched.lock.
{
  pkgs,
  chrome ? true,
}:
let
  lib = pkgs.lib;
  fs = lib.fileset;
  tree = fs.toSource {
    root = ../.;
    fileset = fs.unions (
      [
        ../Cargo.toml
        ../Cargo.lock
        ../crates
        ../examples/clock/Cargo.toml
        ../examples/clock/src
      ]
      ++ lib.optionals chrome [
        ../third_party/rust/winit
        ../third_party/rust/iced_core
        ../third_party/rust/iced_winit
        ../third_party/rust/patch.toml
        ../third_party/rust/Cargo.patched.lock
      ]
    );
  };
  lockFile = if chrome then ../third_party/rust/Cargo.patched.lock else ../Cargo.lock;
  src =
    if chrome then
      pkgs.runCommand "noctalia-iced-src-patched" { } ''
        cp -r ${tree} $out
        chmod -R u+w $out
        # patch.toml's paths are relative to third_party/ (as a --config file); the manifest's are
        # relative to the repository root.
        sed 's|path = "rust/|path = "third_party/rust/|' $out/third_party/rust/patch.toml >> $out/Cargo.toml
        cp $out/third_party/rust/Cargo.patched.lock $out/Cargo.lock
      ''
    else
      tree;
  runtimeLibs = with pkgs; [
    wayland
    libxkbcommon
    vulkan-loader
    libGL
  ];
  clock = pkgs.rustPlatform.buildRustPackage {
    pname = "noctalia-clock-iced";
    version = "0.1.0";
    inherit src;
    buildAndTestSubdir = "examples/clock";
    buildFeatures = lib.optional chrome "wayland-chrome";
    cargoLock.lockFile = lockFile;
    nativeBuildInputs = [
      pkgs.pkg-config
      pkgs.makeWrapper
    ];
    buildInputs = runtimeLibs;
    doCheck = false;
    postInstall = ''
      wrapProgram $out/bin/noctalia-clock-iced --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
    '';
  };
in
pkgs.runCommand (if chrome then "clock-wayland" else "clock-wayland-stock")
  {
    nativeBuildInputs = [
      pkgs.weston
      clock
    ];
    passthru = { inherit clock; };
    FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; };
    __EGL_VENDOR_LIBRARY_DIRS = "${pkgs.mesa}/share/glvnd/egl_vendor.d";
    LIBGL_DRIVERS_PATH = "${pkgs.mesa}/lib/dri";
    LIBGL_ALWAYS_SOFTWARE = "1";
    GALLIUM_DRIVER = "llvmpipe";
    VK_DRIVER_FILES = "${pkgs.mesa}/share/vulkan/icd.d/lvp_icd.${pkgs.stdenv.hostPlatform.linuxArch}.json";
  }
  ''
    export XDG_RUNTIME_DIR=$(mktemp -d) XDG_CACHE_HOME=$(mktemp -d)
    chmod 0700 "$XDG_RUNTIME_DIR"
    mkdir -p $out
    # --debug enables the screenshooter protocol, so the live capture shows the composited output.
    weston --backend=headless --renderer=pixman --socket=wl-0 --width=1280 --height=1700 --idle-time=0 --debug \
      > $out/weston.log 2>&1 &
    weston_pid=$!
    for _ in $(seq 1 200); do [ -S "$XDG_RUNTIME_DIR/wl-0" ] && break; sleep 0.05; done
    export WAYLAND_DISPLAY=wl-0
    fixed="--fixed-time 2026-09-13T10:08:30Z --zone UTC --accent ff5a36"

    status=0
    shot() {
      name=$1; shift
      timeout 120 noctalia-clock-iced $fixed --screenshot $out/$name.png "$@" >> $out/app.log 2>&1 \
        || { echo "$name failed"; status=1; }
    }
    live() {
      name=$1; shift
      noctalia-clock-iced $fixed "$@" >> $out/app.log 2>&1 &
      app=$!
      sleep 6
      (cd $out && weston-screenshooter >> $out/app.log 2>&1 && mv wayland-screenshot-*.png $name.png) \
        || { echo "$name capture failed"; status=1; }
      kill $app 2>/dev/null || true
      wait $app 2>/dev/null || true
    }

    WAYLAND_DEBUG=1 timeout 120 noctalia-clock-iced $fixed --screenshot $out/analog.png \
      > $out/app.log 2> $out/wayland-debug.log || status=1
    shot digital --mode digital
    shot confetti --mode digital --advanced --confetti --after-frames 4
    # Mesa's software rasterizers drop MSAA canvas geometry on surfaces taller than ~1024px
    # (README: Known issues), so the full-height view renders without MSAA.
    shot tall --size 900x1500 --no-antialiasing
    live live-analog
    kill $weston_pid 2>/dev/null || true
    wait $weston_pid 2>/dev/null || true

    grep -E 'set_window_geometry|set_input_region|wl_region.*add|create_buffer|xdg_toplevel.*configure' $out/wayland-debug.log \
      | sed -E 's/^\[[0-9. ]+\] //' | head -40 > $out/protocol.txt || true
    cat $out/app.log
    [ "$status" = 0 ] || { tail -30 $out/weston.log; exit 1; }
  ''
