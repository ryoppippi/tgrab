{
  # Not in nixpkgs — pass a crane lib with the pinned rust-toolchain.toml
  # toolchain from the flake, so the package builds with the same compiler
  # as the dev shell.
  craneLib,
}:
let
  # cleanCargoSource keeps .cargo/config.toml (a *.toml file), which carries
  # the required `--cfg reqwest_unstable` rustflags for impit.
  src = craneLib.cleanCargoSource ./.;

  commonArgs = {
    inherit src;
    strictDeps = true;
  };

  # Build dependencies separately so they are cached across rebuilds
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;
    # Exposed for flake checks (e.g. cargoTest) so they reuse the same
    # source filter and dependency artifacts instead of rebuilding them.
    passthru = { inherit cargoArtifacts commonArgs; };
  }
)
