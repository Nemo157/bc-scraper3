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
  in {
    overlays.default = final: prev: {
      bc-scraper3 = final.callPackage ./package.nix {};
    };
  } // flake-utils.lib.eachSystem systems (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
          self.overlays.default
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
      packages.default = pkgs.bc-scraper3;

      checks = {
        inherit (pkgs) bc-scraper3;
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = [ pkgs.bc-scraper3 ];

        nativeBuildInputs = [ rust-toolchain ];

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
