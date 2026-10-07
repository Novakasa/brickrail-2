{
  description = "bevy flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    # Shared Nix helpers (mkRuntime). Private repo, fetched over SSH.
    conventions.url = "git+ssh://git@github.com/Novakasa/claude-plugins.git";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      conventions,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        inherit (pkgs) lib;
        isLinux = lib.strings.hasInfix "linux" system;

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        # Libraries the built binary needs at run time, on Linux. One list
        # feeds both the devShell's LD_LIBRARY_PATH and `packages.runtime`,
        # so the two cannot drift. Bevy is not built with `dynamic_linking`
        # here, so the toolchain stays out of this list.
        runtimeLibs = lib.optionals isLinux (
          with pkgs;
          [
            # Linked at build time (NEEDED entries of the binary).
            alsa-lib # audio
            libudev-zero # gamepad (gilrs)
            wayland # winit, provides wayland-client
            stdenv.cc.cc.lib # libgcc_s
            # Loaded at run time via dlopen.
            vulkan-loader # wgpu
            libX11
            libXcursor
            libXi
            libXrandr
            libxkbcommon
          ]
        );
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.pkg-config
          ]
          ++ runtimeLibs
          ++ lib.optionals isLinux (
            with pkgs;
            [
              # For debugging around vulkan
              vulkan-tools
              dbus.dev
              uv
              mpremote
            ]
          );
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          LD_LIBRARY_PATH = lib.makeLibraryPath runtimeLibs;
        };

        # Run a cargo-built binary outside the devShell, here or on another
        # machine:
        #   nix run .#runtime -- target/debug/brickrail-client
        #   nix copy --to ssh://<host> .#runtime   # then rsync the binary
        packages.runtime = conventions.lib.mkRuntime {
          inherit pkgs;
          libs = runtimeLibs;
        };
      }
    );
}
