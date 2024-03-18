{
  description = "Rust example flake for Zero to Nix";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.2305.491812.tar.gz";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    let
      # Systems supported
      allSystems = [
        "x86_64-linux"
      ];
    in
    flake-utils.lib.eachSystem allSystems (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        pkgsCross = import nixpkgs {
          inherit system overlays;
          crossSystem = { config = "arm-none"; };
        };
        rustPlatform = with pkgs; makeRustPlatform {
          cargo = rust-bin.stable.latest.minimal;
          rustc = rust-bin.stable.latest.minimal;
        };
        rustPlatformCross = with pkgsCross; makeRustPlatform {
          cargo = rust-bin.stable.latest.minimal;
          rustc = rust-bin.stable.latest.minimal;
        };
      in
      {
        packages.rakuyomi.lua-module = rustPlatform.buildRustPackage {
          name = "rakuyomi-lua-module";
          src = ./.;
          cargoBuildFlags = "-p lua_module";

          cargoLock = {
            lockFile = ./lua_module/Cargo.lock;
          };
          nativeBuildInputs = with pkgs; [ pkg-config openssl libiconv ];
          buildInputs = with pkgs; [ pkg-config openssl libiconv ];
        };
        packages.rakuyomi.kindle-lua-module = rustPlatformCross.buildRustPackage {
          name = "rakuyomi-kindle-lua-module";
          src = ./.;
          cargoBuildFlags = "-p lua_module";

          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = with pkgsCross; [ pkg-config openssl libiconv ];
          buildInputs = with pkgsCross; [ pkg-config openssl libiconv ];
        };
      }
    );
}
