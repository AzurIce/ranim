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
          # (.github/workflows/build.yml pins nightly-2026-08-01).
          p.rust-bin.nightly."2026-08-01".default.override {
            targets = [ "wasm32-unknown-unknown" ];
            extensions = [
              "rust-src"
              "rustfmt"
              "clippy"
            ];
          }
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
        # Building a single package still makes cargo resolve the whole
        # workspace, so every member's manifest *and* its target sources
        # must be present (a declared `[lib]`/`[[bin]]` is validated during
        # manifest parsing); ranim-cli additionally compiles the root `ranim`
        # crate and its path-dep crates, so their sources are included too.
        fileSetForCrate =
          crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions (
              [
                ./Cargo.toml
                ./Cargo.lock
                ./src
                # crane's commonCargoSources only carries Cargo.toml + *.rs;
                # ranim-render embeds its .wgsl shaders via include_wgsl!.
                ./packages/ranim-render/src
              ]
              ++ map craneLib.fileset.commonCargoSources (
                [
                  ./benches
                  ./example-packages/app
                  ./packages/ranim-anims
                  ./packages/ranim-cli
                  ./packages/ranim-core
                  ./packages/ranim-examples
                  ./packages/ranim-items
                  ./packages/ranim-macros
                  ./packages/ranim-render
                  ./xtasks/xtask-examples
                ]
                ++ [ crate ]
              )
            );
          };
        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };
        # Libraries dlopen'd at runtime by winit/wgpu (GUI) and cpal/rodio
        # (preview audio). They never show up in the binary's ELF NEEDED
        # list, so Nix cannot discover them — they have to be wrapped in.
        ranimCliRuntimeLibs = with pkgs; [
          alsa-lib
          libGL
          libX11
          libXcursor
          libXi
          libXrandr
          libxkbcommon
          vulkan-loader
          wayland
        ];

        ranim-cli = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "ranim-cli";
            cargoExtraArgs = "-p ranim-cli";
            src = fileSetForCrate ./packages/ranim-cli;
            meta.mainProgram = "ranim";
            # alsa-sys locates libasound via pkg-config at build time
            nativeBuildInputs = [ pkgs.makeWrapper ] ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.pkg-config ];
            buildInputs =
              individualCrateArgs.buildInputs ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.alsa-lib ];
            postInstall = ''
              wrapProgram "$out/bin/ranim" \
                --prefix PATH : ${lib.makeBinPath [ pkgs.ffmpeg ]} \
                ${lib.optionalString pkgs.stdenv.isLinux ''
                  --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath ranimCliRuntimeLibs}
                ''}
            '';
          }
        );

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
        packages = {
          default = ranim-cli;
          inherit ranim-cli;
        };
        apps =
          let
            # The binary installs as `ranim` (the [[bin]] name), not `ranim-cli`.
            ranimCliApp = {
              type = "app";
              program = "${ranim-cli}/bin/ranim";
            };
          in
          {
            default = ranimCliApp;
            ranim-cli = ranimCliApp;
          };
        devShells.default = craneLib.devShell {
          packages = [
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
            # wasm-bindgen-cli is pinned in ./wasm-bindgen-cli.nix.
            # mdbook-katex
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
            # rodio/cpal preview audio backend (ranim `preview` feature)
            pkgs.alsa-lib
            pkgs.pkg-config
            pkgs.udev
          ];

          shellHook = lib.optionalString pkgs.stdenv.isLinux ''
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
