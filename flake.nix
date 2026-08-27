{
  description = "tgrab";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.treefmt-nix.flakeModule
        inputs.git-hooks.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        {
          config,
          pkgs,
          system,
          ...
        }:
        let
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          # crane with the pinned rust-overlay toolchain
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;

          src = craneLib.cleanCargoSource ./.;

          commonArgs = {
            inherit src;
            strictDeps = true;
          };

          # Build dependencies separately so they are cached across rebuilds
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          # Inject nixpkgs with rust-overlay applied
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ (import inputs.rust-overlay) ];
          };

          treefmt = {
            projectRootFile = "flake.nix";
            settings.global.excludes = [
              ".github/tagpr-template.md"
            ];
            programs = {
              rustfmt = {
                enable = true;
                # Use rustfmt from the pinned toolchain
                package = toolchain;
              };
              oxfmt.enable = true;
              taplo.enable = true;
              deadnix.enable = true;
              statix.enable = true;
              typos.enable = true;
            };
          };

          packages.default = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
            }
          );

          pre-commit.settings = {
            src = ./.;
            package = pkgs.prek;
            hooks = {
              treefmt = {
                enable = true;
                package = config.treefmt.build.wrapper;
              };
              gitleaks = {
                enable = true;
                name = "gitleaks";
                entry = "${pkgs.gitleaks}/bin/gitleaks protect --staged --config .gitleaks.toml";
                language = "system";
                pass_filenames = false;
              };
            };
          };

          checks = {
            build = config.packages.default;
            tests = craneLib.cargoTest (
              commonArgs
              // {
                inherit cargoArtifacts;
              }
            );
          };

          apps.dev = {
            type = "app";
            # Runs `cargo run` with the pinned toolchain and required flags.
            # Usage: nix run .#dev -- <url>
            program = builtins.toString (
              pkgs.writeShellScript "tgrab-dev" ''
                export RUSTFLAGS="--cfg reqwest_unstable"
                exec ${toolchain}/bin/cargo run -- "$@"
              ''
            );
          };

          devShells.default = pkgs.mkShell {
            inherit (config.pre-commit) shellHook;
            nativeBuildInputs = [
              toolchain
              config.treefmt.build.wrapper
            ];
          };
        };
    };
}
