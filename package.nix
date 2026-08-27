{
  lib,
  craneLib,
}:
let
  cargoPackage = (lib.importTOML ./Cargo.toml).package;
  src = craneLib.cleanCargoSource ./.;
  commonArgs = {
    inherit src;
    strictDeps = true;
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;
    passthru = { inherit cargoArtifacts commonArgs; };
    meta = {
      inherit (cargoPackage) description;
      homepage = cargoPackage.repository;
      license = lib.getLicenseFromSpdxId cargoPackage.license;
      mainProgram = cargoPackage.name;
      maintainers = [ lib.maintainers.ryoppippi ];
      platforms = import ./systems.nix;
    };
  }
)
