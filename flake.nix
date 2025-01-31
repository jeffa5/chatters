{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=master";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = [
        pkgs.rustc
        pkgs.cargo
        pkgs.rustfmt
        pkgs.clippy

        pkgs.openssl
        pkgs.pkg-config
      ];

      PROTOC = "${pkgs.protobuf}/bin/protoc";
    };
  };
}
