# Pure Cargo workspace; Sunshine and Moonlight are separate processes and
# are NOT linked here, so the GPL of those upstreams does not constrain
# this crate tree beyond its own GPL-3.0 license.
{
  lib,
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "airglow";
  version = (lib.importTOML ./Cargo.toml).workspace.package.version;

  src = lib.sourceByRegex ./. [
    "Cargo.toml"
    "Cargo.lock"
    "crates(/.*)?"
  ];

  cargoLock.lockFile = ./Cargo.lock;

  cargoBuildFlags = [ "--workspace" ];

  # Default `cargo install` does not handle workspace roots; copy the
  # built binaries ourselves (target dir is <target>/<profile>/).
  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    find target -maxdepth 3 -type f -name 'airglow*' ! -name '*.d' \
      -exec install -m755 {} $out/bin/ \;
    runHook postInstall
  '';

  meta = {
    description = "GDM-integrated remote desktop sessions via Sunshine/Moonlight";
    license = lib.licenses.gpl3Only;
    mainProgram = "airglow-hostd";
    platforms = lib.platforms.linux;
  };
}
