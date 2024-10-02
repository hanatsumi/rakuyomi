{
  description = "Rust example flake for Zero to Nix";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.0.tar.gz";
    nixpkgs-patched-koreader.url = "github:ekisu/nixpkgs/koreader-202407";
    crane.url = "github:ipetkov/crane";
    flake-utils.url  = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, crane, nixpkgs, nixpkgs-patched-koreader, flake-utils, rust-overlay }:
    let
      genericVersion = self.lastModifiedDate + "-" + (self.shortRev or self.dirtyShortRev);
      semanticReleaseVersion = builtins.getEnv "SEMANTIC_RELEASE_VERSION";
      version = if semanticReleaseVersion != "" then semanticReleaseVersion else genericVersion;
    in flake-utils.lib.eachDefaultSystem (system:
      let
        # FIXME probably `armv7-unknown-linux-gnueabihf` is more accurate
        desktopTarget = "x86_64-unknown-linux-musl";
        kindleTarget = "arm-unknown-linux-musleabi";

        pkgs = import nixpkgs {
          inherit system;
        };

        pkgs-openssl-patched-koreader = import nixpkgs-patched-koreader {
          inherit system;
          config = {
            permittedInsecurePackages = [ "openssl-1.1.1w" ];
          };
        };

        patchedKoreader = pkgs-openssl-patched-koreader.koreader.overrideAttrs (oldAttrs: {
          patches = [./patches/fontlist-use-bitser.patch];
        });

        buildBackendRustPackage = {
          packageName,
          copyTarget ? false
        }: target:
          let
            pkgs = import nixpkgs {
              inherit system;
              config.allowUnsupportedSystem = true;
              overlays = [ (import rust-overlay) ];
            };

            pkgsCross = import nixpkgs {
              inherit system;
              config.allowUnsupportedSystem = true;
              crossSystem.config = target;
            };

            craneLib = (crane.mkLib pkgs).overrideToolchain (p: p.rust-bin.stable.latest.default.override {
              targets = [target];
            });
          in
            with pkgs; craneLib.buildPackage rec {
              doInstallCargoArtifacts = copyTarget;
              installCargoArtifactsMode = "use-symlink";

              doCheck = false;

              src = ./backend;
              cargoExtraArgs = "--package=${packageName}";

              nativeBuildInputs = [
                stdenv.cc
              ];

              TARGET_CC = with pkgsCross.stdenv; "${cc}/bin/${cc.targetPrefix}cc";
              CARGO_BUILD_TARGET = target;
              # https://github.com/rust-lang/cargo/issues/4133
              CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static -C linker=${TARGET_CC}";
            };

          mkServerPackage = buildBackendRustPackage { packageName = "server"; };

          mkCliPackage = buildBackendRustPackage { packageName = "cli"; copyTarget = true; };

          versionFile = pkgs.writeText "VERSION" version;

          mkPluginFolder = target:
            let
              server = mkServerPackage target;
            in
              with pkgs; stdenv.mkDerivation {
                name = "rakuyomi-plugin";
                src = ./frontend;
                phases = [ "unpackPhase" "installPhase" ];
                installPhase = ''
                  mkdir $out
                  cp -r $src/rakuyomi.koplugin/* $out/
                  cp ${server}/bin/server $out/server
                  cp ${server}/bin/uds_http_request $out/uds_http_request
                  cp ${versionFile} $out/VERSION
                '';
              };
          
          mkKoreaderWithRakuyomi = target:
            let
              plugin = mkPluginFolder target;
            in
              patchedKoreader.overrideAttrs (finalAttrs: previousAttrs: {
                installPhase = previousAttrs.installPhase + ''
                  ln -sf ${plugin} $out/lib/koreader/plugins/rakuyomi.koplugin
                '';
              });
          
          # FIXME this is really bad and relies on `mkCliPackage` copying the _entire_
          # target folder to the nix store (which is really bad too)
          mkSchemaFile = target:
            let
              cli = mkCliPackage target;
            in
              with pkgs; stdenv.mkDerivation {
                name = "rakuyomi-settings-schema";
                phases = [ "installPhase" ];
                installPhase = ''
                  cp ${cli}/target/${target}/release/settings.schema.json $out
                '';
              };
      in
      {
        packages.rakuyomi.desktop = mkPluginFolder desktopTarget;
        packages.rakuyomi.koreader-with-plugin = mkKoreaderWithRakuyomi desktopTarget;
        packages.rakuyomi.kindle = mkPluginFolder kindleTarget;
        packages.rakuyomi.cli = mkCliPackage desktopTarget;
        packages.rakuyomi.settings-schema = mkSchemaFile desktopTarget;
      }
    );
}
