{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
  let
    systems = builtins.filter
      (system: nixpkgs.lib.strings.hasSuffix "linux" system)
      flake-utils.lib.defaultSystems;
  in flake-utils.lib.eachSystem systems (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
        ];
      };

      rust-toolchain  = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
        toolchain.minimal.override {
          extensions = [
            "rust-src"
            "rustc-codegen-cranelift-preview"
            "clippy"
            "rustfmt"
          ];
        }
      );
    in {
      devShells.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          fontconfig
          pkg-config
          rust-toolchain
        ];

        buildInputs = with pkgs; [
          alsa-lib
          libxkbcommon
          udev
          wayland
          wayland-protocols
        ];

        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
          libglvnd
          libxkbcommon
          vulkan-loader
          wayland
        ]);
      };
    }
  );
}
