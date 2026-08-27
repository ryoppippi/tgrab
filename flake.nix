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

      systems = import ./systems.nix;

      perSystem =
        {
          config,
          pkgs,
          system,
          ...
        }:
        let
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;
        in
        {
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

          packages = {
            tgrab = pkgs.callPackage ./package.nix { inherit craneLib; };
            default = config.packages.tgrab;
          };

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
              config.packages.default.passthru.commonArgs
              // {
                inherit (config.packages.default.passthru) cargoArtifacts;
              }
            );
          };

          apps.dev = {
            type = "app";
            # Runs `cargo run` with the pinned toolchain and required flags.
            # Usage: nix run .#dev -- <url>
            program = toString (
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
