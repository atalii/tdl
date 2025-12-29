{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = with pkgs; [
    ffmpeg
    cargo
    rustc
    rustfmt
    rust-analyzer
    pkg-config
    openssl
  ];
}
