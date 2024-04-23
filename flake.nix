{
  description = "Rust example flake for Zero to Nix";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.0.tar.gz";
    nixpkgs-patched-koreader.url = "github:ekisu/nixpkgs/koreader-add-openssl-dependency";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-patched-koreader, naersk, fenix, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # FIXME probably `armv7-unknown-linux-gnueabihf` is more accurate
        desktopTarget = "x86_64-unknown-linux-musl";
        kindleTarget = "arm-unknown-linux-musleabi";

        pkgs = import nixpkgs {
          inherit system;
        };

        pkgs-patched-koreader = import nixpkgs-patched-koreader {
          inherit system;
          config = {
            permittedInsecurePackages = [ "openssl-1.1.1w" ];
          };
        };

        buildBackendRustPackage = {
          packageName,
          copyTarget ? false
        }: target:
          let
            pkgsCross = import nixpkgs {
              inherit system;
              config.allowUnsupportedSystem = true;
              crossSystem.config = target;
            };
            toolchain = with fenix.packages.${system};
              combine [
                minimal.rustc
                minimal.cargo
                targets.${target}.latest.rust-std
              ];
            naersk' = pkgs.callPackage naersk {
              cargo = toolchain;
              rustc = toolchain;
            };
          in
            naersk'.buildPackage rec {
              inherit copyTarget;

              compressTarget = false;
              src = ./backend;
              cargoBuildOptions = defaultOptions: defaultOptions ++ ["-p" packageName];

              CARGO_BUILD_TARGET = target;
              TARGET_CC = with pkgsCross.stdenv; "${cc}/bin/${cc.targetPrefix}cc";
              CARGO_BUILD_RUSTFLAGS = [
                "-C" "target-feature=+crt-static"
                # https://github.com/rust-lang/cargo/issues/4133
                "-C" "linker=${TARGET_CC}"
              ];
            };

          mkServerPackage = buildBackendRustPackage { packageName = "server"; };

          mkCliPackage = buildBackendRustPackage { packageName = "cli"; copyTarget = true; };

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
                '';
              };
          
          mkKoreaderWithRakuyomi = target:
            let
              plugin = mkPluginFolder target;
            in
              with pkgs-patched-koreader; koreader.overrideAttrs (finalAttrs: previousAttrs: {
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
