{
  description = "ranim";

  # nixConfig = {
  #   extra-substituters = [
  #     "https://mirrors.ustc.edu.cn/nix-channels/store"
  #   ];
  #   trusted-substituters = [
  #     "https://mirrors.ustc.edu.cn/nix-channels/store"
  #   ];
  # };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      crane,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        inherit (pkgs) lib;
        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p:
          p.rust-bin.selectLatestNightlyWith (
            toolchain:
            toolchain.default.override {
              targets = [ "wasm32-unknown-unknown" ];
              extensions = [ "rust-src" ];
            }
          )
        );
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = [
            # Add additional build inputs here
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
        };
        # Build *just* the cargo dependencies (of the entire workspace),
        # so we can reuse all of that work (e.g. via cachix) when running in CI
        # It is *highly* recommended to use something like cargo-hakari to avoid
        # cache misses when building individual top-level-crates
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        fileSetForCrate =
          crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              (craneLib.fileset.commonCargoSources ./packages/ranim-cli)
              (craneLib.fileset.commonCargoSources ./packages/ranim-macros)
              (craneLib.fileset.commonCargoSources crate)
            ];
          };
        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };
        ranim-cli = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "ranim-cli";
            cargoExtraArgs = "-p ranim-cli";
            src = fileSetForCrate ./packages/ranim-cli;
          }
        );

        puffin_viewer = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "puffin_viewer";
          version = "0.22.0";

          cargoBuildFlags = [ "-p puffin_viewer" ];
          cargoPatches = [ ./puffin-Cargo.lock.patch ];

          src = pkgs.fetchFromGitHub {
            owner = "EmbarkStudios";
            repo = "puffin";
            rev = "puffin_viewer-0.22.0";
            hash = "sha256-ppE/f6jLRe6a1lfUQUlxTq/L29DwAD/a58u5utUJMoU=";
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.gtk3 ];

          cargoHash = "sha256-zhijQ+9vVB4IL/t1+IGLAnvJka0AB1yJRWo/qEyUfx0=";
        });

        mdbook-typst-math = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "mdbook-typst-math";
          version = "0.3.0-unstable-2026-08-25";

          src = pkgs.fetchFromGitHub {
            owner = "duskmoon314";
            repo = "mdbook-typst-math";
            rev = "e310ec82ecaec5ae8e516ac07e9cab85fb506bc3";
            hash = "sha256-DQBUBjb1eNTmHj35sjuaN07GW4rpDlM40GFB3dWtiHg=";
          };

          cargoHash = "sha256-Mms8HDxak46oTbcICEjw2612gHGdsrO9kVLQTlNi24o=";
        });
      in
      {
        packages = { inherit ranim-cli; };
        apps = {
          ranim-cli = flake-utils.lib.mkApp {
            drv = ranim-cli;
          };
        };
        devShells.default = craneLib.devShell {
          packages = [
            puffin_viewer
            mdbook-typst-math
          ]
          ++ (with pkgs; [
            git-cliff
            # cargo-release
            cargo-edit
            samply
            cargo-udeps
            miniserve
            trunk
            zola
            mdbook
            wasm-pack
            binaryen
            mdbook-mermaid
            typst
            gh
            ffmpeg
            # wasm-bindgen-cli_0_2_106
            # mdbook-katex
            # wasm-bindgen-cli
            # mdbook-i18n-helpers
          ])
          ++ [
            (pkgs.callPackage ./cargo-release.nix { })
            (pkgs.callPackage ./wasm-bindgen-cli.nix { })
          ]
          ++ lib.optionals pkgs.stdenv.isLinux [
            pkgs.vulkan-loader
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.libX11
          ];

          shellHook = lib.optionalString pkgs.stdenv.isLinux ''
            export LD_LIBRARY_PATH="${lib.makeLibraryPath [
              pkgs.vulkan-loader
              pkgs.wayland
              pkgs.libxkbcommon
              pkgs.libX11
            ]}''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          '';
        };
      }
    );
}
