{
  description = "noctalia-iced: Noctalia's design language for iced, with a clock demo";

  # The nixpkgs revision libnoctalia-ui's checks run against.
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/eaad089433ca2bb662274377d33df3d0e51ef28b";

  outputs =
    { self, nixpkgs }:
    let
      linuxSystems = [
        "aarch64-linux"
        "x86_64-linux"
      ];
      forLinux = f: nixpkgs.lib.genAttrs linuxSystems (system: f nixpkgs.legacyPackages.${system});
      clockCheck = pkgs: chrome: import ./nix/wayland-check.nix { inherit pkgs chrome; };
    in
    {
      packages = forLinux (pkgs: rec {
        # crates.io iced and winit.
        noctalia-clock-iced = (clockCheck pkgs false).passthru.clock;
        # `wayland-chrome`, with the patched crates in third_party/rust.
        noctalia-clock-iced-chrome = (clockCheck pkgs true).passthru.clock;
        default = noctalia-clock-iced;
      });

      checks = forLinux (pkgs: {
        clock-wayland = clockCheck pkgs true;
        clock-wayland-stock = clockCheck pkgs false;
        chrome-probe = import ./third_party/rust/chrome-probe/wayland-check.nix { inherit pkgs; };
      });

      devShells = forLinux (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            pkg-config
            weston
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              wayland
              libxkbcommon
              vulkan-loader
              libGL
            ]
          );
        };
      });
    };
}
