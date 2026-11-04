{ pkgs ? import <nixpkgs> {} }:
let
  libs = with pkgs; [
    cargo
    libx11 libxcursor libxrandr libxi
    libxinerama libxext libxrender libxfixes
    libxcb libxkbcommon fontconfig freetype wayland libGL
  ];
in pkgs.mkShell {
  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = libs;
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libs;
}
