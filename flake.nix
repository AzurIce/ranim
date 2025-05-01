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
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust-tools = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" ];
        };
        puffin_viewer = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "puffin_viewer";
          version = "0.22.0";

          cargoBuildFlags = [ "-p puffin_viewer" ];
          cargoPatches = [
            ./puffin-Cargo.lock.patch
          ];

          src = pkgs.fetchFromGitHub {
            owner = "EmbarkStudios";
            repo = "puffin";
            rev = "puffin_viewer-0.22.0";
            hash = "sha256-ppE/f6jLRe6a1lfUQUlxTq/L29DwAD/a58u5utUJMoU=";
          };

          cargoHash = "sha256-zhijQ+9vVB4IL/t1+IGLAnvJka0AB1yJRWo/qEyUfx0=";
        });
      in
      {
        devShells.default = pkgs.mkShell {
          # prioritize system clang, see https://github.com/zed-industries/zed/issues/7036
          # https://github.com/gfx-rs/gfx/issues/2309
          # https://mac.install.guide/commandlinetools/7
          shellHook = ''
            export PATH=/usr/bin:$PATH
          '';

          buildInputs = with pkgs; [
            clang
            llvmPackages_17.bintools
            libusb1
            openssl
            pkg-config
          ] ++ [
            rust-tools
          ] ++ (with pkgs.darwin.apple_sdk.frameworks; pkgs.lib.optionals pkgs.stdenv.isDarwin [
            System
            IOKit
            Security
            CoreFoundation
            AppKit
          ]);
          packages = [
            puffin_viewer
          ];
        };
      }
    );
}
