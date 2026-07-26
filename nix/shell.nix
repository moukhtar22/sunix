{ cargo
, clippy
, gtk4
, gtk4-layer-shell
, lib
, mkShell
, pkg-config
, rustc
, rustfmt
}:

let
  buildInputs = [ gtk4 gtk4-layer-shell ];
in
mkShell {
  inherit buildInputs;

  LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;

  nativeBuildInputs = [
    cargo
    clippy
    pkg-config
    rustc
    rustfmt
  ];
}
