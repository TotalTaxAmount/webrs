{ pkgs ? (import <nixpkgs> {
    config.allowUnfree = true;
}) }:

pkgs.stdenv.mkDerivation {
  name = "rust";

  buildInputs = with pkgs; [
    rustup
    cargo
    cargo-flamegraph
  ];
}
