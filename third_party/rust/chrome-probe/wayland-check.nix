# Builds chrome-probe against the patched crates in third_party/rust and runs it under headless
# weston twice (floating, then maximized), keeping the Wayland protocol trace of each run.
#
#   scripts/linux.sh chrome-probe        (flake check checks.<system>.chrome-probe)
{ pkgs }:
let
  src = builtins.path {
    name = "third-party-rust";
    path = ../.;
    filter = path: _: !(builtins.elem (baseNameOf path) [ "target" "out" "result" "patches" ]);
  };
  runtimeLibs = with pkgs; [
    wayland
    libxkbcommon
    vulkan-loader
    libGL
  ];
  probe = pkgs.rustPlatform.buildRustPackage {
    pname = "chrome-probe";
    version = "0.0.0";
    inherit src;
    cargoRoot = "chrome-probe";
    buildAndTestSubdir = "chrome-probe";
    cargoLock.lockFile = ./Cargo.lock;
    nativeBuildInputs = [
      pkgs.pkg-config
      pkgs.makeWrapper
    ];
    buildInputs = runtimeLibs;
    doCheck = false;
    postInstall = ''
      wrapProgram $out/bin/chrome-probe --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtimeLibs}
    '';
  };
in
pkgs.runCommand "chrome-probe-wayland"
  {
    nativeBuildInputs = [
      pkgs.weston
      probe
    ];
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
    weston --backend=headless --renderer=pixman --socket=nui-0 --width=1280 --height=900 --idle-time=0 \
      > $out/weston.log 2>&1 &
    weston_pid=$!
    for _ in $(seq 1 200); do
      [ -S "$XDG_RUNTIME_DIR/nui-0" ] && break
      sleep 0.05
    done
    export WAYLAND_DISPLAY=nui-0

    status=0
    run() {
      name=$1
      shift
      mkdir -p $out/$name
      WAYLAND_DEBUG=1 timeout 60 chrome-probe "$@" --screenshot $out/$name/probe.png \
        > $out/$name/app.log 2> $out/$name/wayland-debug.log || return $?
      grep -E 'set_window_geometry|set_input_region|wl_region#[0-9]+\.add|xdg_toplevel#[0-9]+\.(configure|set_maximized|set_min_size|set_max_size)|wl_shm_pool#[0-9]+\.create_buffer|wl_surface#[0-9]+\.(attach|set_buffer_scale)|wp_viewport#[0-9]+\.set_destination' \
        $out/$name/wayland-debug.log > $out/$name/protocol.txt || true
      [ -s $out/$name/probe.png ]
    }
    run floating || status=$?
    run maximized --maximize-after-first-chrome || status=$?

    kill $weston_pid 2>/dev/null || true
    wait $weston_pid 2>/dev/null || true
    for name in floating maximized; do
      echo "--- $name"
      cat $out/$name/app.log || true
    done
    if [ "$status" != 0 ]; then
      echo "probe failed (status $status)"
      tail -40 $out/weston.log
      exit 1
    fi
  ''
