{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  packages = with pkgs; [
    rustc
    cargo
    rust-analyzer
    rustfmt
    clippy
  ];
}
