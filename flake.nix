{
  description = "A simple flake for rust development";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    # Useful lib for caching cargo builds
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Provide toolchain profiles for rust
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Don't really know
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      naersk,
      fenix,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        buildToolchain =
          with fenix.packages.${system};
          combine ([
            minimal.rustc
            minimal.cargo
          ]);

        devToolchain =
          with fenix.packages.${system};
          combine [
            (complete.withComponents [
              "cargo"
              "clippy"
              "rust-src"
              "rustc"
              "rustfmt"
            ])
          ];

        naerskLib = naersk.lib.${system}.override {
          cargo = buildToolchain;
          rustc = buildToolchain;
        };

        unixBuildDeps = with pkgs; [ ];
      in
      rec {
        packages = {
          linux = naerskLib.buildPackage {
            src = ./.;
            nativeBuildInputs = unixBuildDeps;
          };
        };

        defaultPackage = packages.linux;

        # Personal Development shell :D
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = unixBuildDeps;
          buildInputs = [
            # Required
            devToolchain
            pkgs.rust-analyzer
            pkgs.cargo-watch
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath unixBuildDeps}";
          '';
        };
      }
    );
}
