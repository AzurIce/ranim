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
          # Keep in sync with the CI lint/build jobs
          # (.github/workflows/build.yml pins nightly-2026-10-01).
          p.rust-bin.nightly."2026-10-01".default.override {
            targets = [ "wasm32-unknown-unknown" ];
            extensions = [
              "rust-src"
              "rustfmt"
              "clippy"
            ];
          }
        );
        mdbook-typst-math = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "mdbook-typst-math";
          version = "0.4.0";

          src = pkgs.fetchFromGitHub {
            owner = "duskmoon314";
            repo = "mdbook-typst-math";
            rev = "52341bd8ec2ffde6db92cfd5e5d7503076fbd94e";
            hash = "sha256-0tBtRI3R19f4UA6uEzRZoKgptc/OgUEOOYm6hammweo=";
          };

          cargoHash = "sha256-PMgo5PnDPUQgLwpN3S6N7tFYXv2Jb0FTepf7HOHXpjY=";
        });
      in
      {
        devShells.default = craneLib.devShell {
          packages = [
            mdbook-typst-math
          ]
          ++ (with pkgs; [
            git-cliff
            cargo-release
            cargo-edit
            samply
            cargo-udeps
            miniserve
            zola
            mdbook
            binaryen
            mdbook-mermaid
            typst
            gh
            ffmpeg
            wasm-bindgen-cli_0_2_129
            # Match the wasm-bindgen version in Cargo.lock.
            # mdbook-katex
            # mdbook-i18n-helpers
          ])
          ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            pkgs.vulkan-loader
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.libX11
            # rodio/cpal preview audio backend (ranim `preview` feature)
            pkgs.alsa-lib
            pkgs.pkg-config
            pkgs.udev
          ];

          shellHook = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            export LD_LIBRARY_PATH="${
              lib.makeLibraryPath [
                pkgs.vulkan-loader
                pkgs.wayland
                pkgs.libxkbcommon
                pkgs.libX11
              ]
            }''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
          '';
        };
      }
    );
}
