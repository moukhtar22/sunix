{ gtk4
, gtk4-layer-shell
, lib
, pkg-config
, rustPlatform
, wrapGAppsHook4
}:

let
  sourceRoot = toString ../.;

  includedFiles = [
    "Cargo.lock"
    "Cargo.toml"
    "LICENSE"
    "README.md"
  ];

  includedDirs = [
    "assets"
    "src"
  ];

  source = lib.cleanSourceWith {
    filter = path: _type:
      let
        pathString = toString path;
        relativePath = lib.removePrefix "${sourceRoot}/" pathString;
        topLevel = builtins.head (lib.splitString "/" relativePath);
      in
      pathString == sourceRoot
      || lib.elem relativePath includedFiles
      || lib.elem topLevel includedDirs;
    src = ../.;
  };
in
rustPlatform.buildRustPackage {
  buildInputs = [
    gtk4
    gtk4-layer-shell
  ];

  cargoLock.lockFile = ../Cargo.lock;

  meta = with lib; {
    description = "GTK4 layer-shell popup for NixOS & Home Manager Software Update summaries";
    homepage = "https://github.com/gvolpe/sunix";
    license = licenses.mit;
    mainProgram = "sunix";
    maintainers = with maintainers; [ gvolpe ];
    platforms = platforms.linux;
  };

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];

  pname = "sunix";
  src = source;
  version = "0.1.0";
}
