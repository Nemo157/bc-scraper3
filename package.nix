{
  lib,
  rustPlatform,

  alsa-lib,
  fontconfig,
  libxkbcommon,
  openssl,
  pkg-config,
  udev,
  vulkan-loader,
  wayland,
  wayland-protocols,
}:

rustPlatform.buildRustPackage {
  pname = "bc-scraper3";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.difference ./. ./.cargo;
  };
  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [
    fontconfig
    pkg-config
  ];

  buildInputs = [
    alsa-lib
    libxkbcommon
    udev
    wayland
    wayland-protocols
    openssl
  ];

  postFixup = ''
    patchelf $out/bin/bc-scraper3 --add-rpath ${lib.makeLibraryPath [ libxkbcommon vulkan-loader ]}
  '';
}
