let
  nixpkgs = builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/e7439b6b14ad3cc35d05608ebca9bce01a25f5f8.tar.gz";
    sha256 = "sha256-H83yz/do+hjOO0uujHyH8RgtIQP2v7Mw+cLwcZK/xKc=";
  };
  pkgs = import nixpkgs { };
in
pkgs.mkShell {
  packages = with pkgs; [ gcc pkg-config z3 llvmPackages.libclang python3 ];
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  LD_LIBRARY_PATH = "${pkgs.z3.lib}/lib";
}
