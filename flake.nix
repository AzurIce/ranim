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
    notist.url = "github:AzurIce/notist";
    # Includes the workspace hotpatch cascade fix missing from dx 0.7.10.
    dioxus = {
      url = "github:DioxusLabs/dioxus/7bd964ae9469daafcb76b73556162d9082be255b";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.crane.follows = "crane";
      inputs.rust-overlay.follows = "rust-overlay";
    };
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
      notist,
      dioxus,
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
        # The pinned upstream package includes the HTML highlighting assets.
        notistPackage = notist.packages.${system}.notist;
        dioxusPackage = dioxus.packages.${system}.dioxus-cli;
      in
      {
        packages.notist = notistPackage;
        packages.dioxus-cli = dioxusPackage;
        devShells.default = craneLib.devShell {
          packages = [
            notistPackage
            dioxusPackage
          ]
          ++ (with pkgs; [
            git-cliff
            cargo-release
            cargo-edit
            samply
            cargo-udeps
            miniserve
            binaryen
            typst
            gh
            ffmpeg
            wasm-bindgen-cli_0_2_129
            # Match the wasm-bindgen version in Cargo.lock.
          ])
          ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            pkgs.vulkan-loader
            pkgs.wayland
            pkgs.libxkbcommon
            pkgs.libX11
            # rodio/cpal preview audio backend (ranim `cmd-preview` feature)
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
