{
  description = "Payments processing engine";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    git-hooks-nix = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.git-hooks-nix.flakeModule ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        {
          config,
          pkgs,
          lib,
          ...
        }:
        {
          pre-commit.settings = {
            excludes = [ "^target/" ];

            hooks = {
              # Pre-commit: format with nightly rustfmt
              # Custom entry needed because +nightly must come before fmt
              rustfmt-nightly = {
                enable = true;
                name = "rustfmt-nightly";
                description = "Format Rust code with nightly rustfmt";
                entry = "cargo +nightly fmt --";
                types = [ "rust" ];
                pass_filenames = true;
                stages = [ "pre-commit" ];
              };

              # Pre-push: run clippy with same args as CI
              clippy = {
                enable = true;
                name = "clippy";
                description = "Lint Rust code with clippy";
                entry = "cargo clippy --all-targets --all-features -- -D warnings";
                types = [ "rust" ];
                pass_filenames = false;
                stages = [ "pre-push" ];
              };
            };
          };

          devShells.default = pkgs.mkShell {
            shellHook = config.pre-commit.installationScript;

            packages = with pkgs; [
              # Rust toolchain
              rustc
              cargo
              rustfmt
              clippy

              # Testing
              cargo-nextest
              cargo-llvm-cov

              # Nix formatting
              nixfmt-rfc-style
            ];

            # Match CI environment
            CARGO_TERM_COLOR = "always";
            RUSTFLAGS = "-Dwarnings";
          };
        };
    };
}
