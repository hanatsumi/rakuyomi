{ lib, stdenv
, fetchurl
, makeWrapper
, fetchFromGitHub
, dpkg
, glib
, gnutar
, gtk3-x11
, openssl_1_1
, luajit
, sdcv
, SDL2
, unzip
, p7zip
, plugins ? [] }:
let
  # FIXME we can't really fetch a version here
  macosSrc = ./koreader-macos-arm64.zip;

  linuxSrc = version: if stdenv.isAarch64 then fetchurl {
    url = "https://github.com/koreader/koreader/releases/download/v${version}/koreader-${version}-arm64.deb";
    hash = "sha256-KrkY1lTwq8mIomUUCQ9KvfZqinJ74Y86fkPexsFiOPg=";
  } else fetchurl {
    url = "https://github.com/koreader/koreader/releases/download/v${version}/koreader-${version}-amd64.deb";
    hash = "sha256-ibehFrOcJqhM+CMAcHDn3Xwy6CueB8kdnoYMMDe/2Js=";
  };

  luajit_lua52 = luajit.override { enable52Compat = true; };
in
stdenv.mkDerivation rec {
  pname = "koreader";
  version = "2024.11";

  src = if stdenv.isDarwin then macosSrc else (linuxSrc version);

  src_repo = fetchFromGitHub {
    repo = "koreader";
    owner = "koreader";
    rev = "v${version}";
    fetchSubmodules = true;
    sha256 = "sha256-gHn1xqBc7M9wkek1Ja1gry8TKIuUxQP8T45x3z2S4uc=";
  };

  sourceRoot = ".";
  nativeBuildInputs = [ makeWrapper ]
    ++ (lib.optionals stdenv.isLinux [ dpkg ])
    ++ (lib.optionals stdenv.isDarwin [ unzip p7zip ]);

  buildInputs = [
    glib
    gnutar
    gtk3-x11
    luajit_lua52
    openssl_1_1
    sdcv
    SDL2
  ];

  unpackCmd = lib.optionalString stdenv.isLinux "dpkg-deb -x ${src} .";

  dontConfigure = true;
  dontBuild = true;

  patches = lib.optionals stdenv.isLinux [ ./patches/datastorage-isolate-storage.patch ];

  installPhase = let
    copyFiles = if stdenv.isLinux then ''
      cp -R usr/* $out/
    '' else ''
      mkdir -p $out/Applications
      7z x *.7z
      cp -R KOReader.app $out/Applications/
      ls -l $out/Applications
    '';

    koreaderFolder = if stdenv.isDarwin
      then "$out/Applications/KOReader.app/Contents/koreader"
      else "$out/lib/koreader";
    
    wrapBinary = lib.optionalString stdenv.isLinux ''
      wrapProgram $out/bin/koreader --prefix LD_LIBRARY_PATH : ${
        lib.makeLibraryPath [ gtk3-x11 SDL2 glib openssl_1_1 stdenv.cc.cc ]
      }
    '';

    installPlugins = lib.strings.concatMapStringsSep
      "\n"
      (plugin: "cp -R ${plugin} ${koreaderFolder}/plugins/${plugin.name}.koplugin")
      plugins;
  in ''
    set -v
    runHook preInstall
    mkdir -p $out
    ${copyFiles}
    ln -sf ${luajit_lua52}/bin/luajit ${koreaderFolder}/luajit
    ln -sf ${sdcv}/bin/sdcv ${koreaderFolder}/sdcv
    ln -sf ${gnutar}/bin/tar ${koreaderFolder}/tar
    find ${src_repo}/resources/fonts -type d -execdir cp -r '{}' ${koreaderFolder}/fonts \;
    find $out -xtype l -print -delete
    ${wrapBinary}
    ${installPlugins}
    runHook postInstall
  '';

  postFixup = lib.optionalString stdenv.isDarwin ''
    mkdir -p $out/bin
    cat > $out/bin/koreader <<EOF
    #!/bin/sh
    exec $out/Applications/KOReader.app/Contents/MacOS/koreader "\$@"
    EOF
    chmod +x $out/bin/koreader
  '';

  meta = with lib; {
    homepage = "https://github.com/koreader/koreader";
    description =
      "An ebook reader application supporting PDF, DjVu, EPUB, FB2 and many more formats, running on Cervantes, Kindle, Kobo, PocketBook and Android devices";
    mainProgram = "koreader";
    sourceProvenance = with sourceTypes; [ binaryNativeCode ];
    platforms = [ "aarch64-linux" "x86_64-linux" "aarch64-darwin" ];
    license = licenses.agpl3Only;
    maintainers = with maintainers; [ contrun neonfuz ];
  };
}
