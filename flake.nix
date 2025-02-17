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
    customBuildRustCrateForPkgs = pkgs:
      pkgs.buildRustCrate.override {
        defaultCrateOverrides =
          pkgs.defaultCrateOverrides
          // {
            libsignal-protocol = attrs: {
              buildInputs = [pkgs.protobuf];
            };
            libsignal-service = attrs: {
              buildInputs = [pkgs.protobuf];
            };
            presage-store-sled = attrs: {
              buildInputs = [pkgs.protobuf];
            };
            chatters-signal = attrs: {
              buildInputs = [
                pkgs.pkg-config
                pkgs.librandombytes
                pkgs.openssl
              ];
            };
            chatters-lib = attrs: {
              buildInputs = [
                pkgs.pkg-config
                pkgs.librandombytes
                pkgs.openssl
              ];
            };
            pqcrypto-kyber = attrs: {
              buildInputs = [
                pkgs.pkg-config
                pkgs.librandombytes
                pkgs.openssl
              ];
            };
          };
      };
    cargoNix = pkgs.callPackage ./Cargo.nix {
      release = false;
      buildRustCrateForPkgs = customBuildRustCrateForPkgs;
    };
  in {
    packages.${system} = rec {
      chatters-local = cargoNix.workspaceMembers.chatters-local.build;
      chatters-signal = cargoNix.workspaceMembers.chatters-signal.build;
      chatters-matrix = cargoNix.workspaceMembers.chatters-matrix.build;
      chatters = pkgs.symlinkJoin {
        name = "chatters";
        paths = [
          chatters-local
          chatters-signal
          chatters-matrix
        ];
      };
    };

    devShells.${system}.default = pkgs.mkShell {
      packages = [
        pkgs.rustc
        pkgs.cargo
        pkgs.rustfmt
        pkgs.clippy
        pkgs.cargo-insta

        pkgs.crate2nix

        pkgs.openssl
        pkgs.pkg-config
        pkgs.sqlite
      ];

      PROTOC = "${pkgs.protobuf}/bin/protoc";
    };
  };
}
