let
  moz_overlay = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
  nixpkgs = import <nixpkgs> { overlays = [ moz_overlay ]; };
  ruststable = (nixpkgs.latest.rustChannels.stable.rust.override { extensions = [ "rust-src" "rls-preview" "rust-analysis" "rustfmt-preview" ];});
in

    with nixpkgs;
  nixpkgs.mkShell {
    name = "rust";
    buildInputs = [ openssl pkgconfig nasm rustup ruststable cmake zlib python3 ];
  }
