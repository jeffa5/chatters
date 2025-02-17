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
    wrap-chatters = {
      pkg,
      name,
    }:
      pkgs.stdenvNoCC.mkDerivation {
        inherit name;
        src = ./.;
        buildPhase = ''
          mkdir $out
          cp -r ${pkg.build}/* $out/.
        '';
        installPhase = ''
          mkdir -p $out/share/${name}
          cp $src/config.toml $out/share/${name}/config.toml
        '';
      };
    chatters-local = wrap-chatters {
      pkg = cargoNix.workspaceMembers.chatters-local;
      name = "chatters-local";
    };
    chatters-signal = wrap-chatters {
      pkg = cargoNix.workspaceMembers.chatters-signal;
      name = "chatters-signal";
    };
    chatters-matrix = wrap-chatters {
      pkg = cargoNix.workspaceMembers.chatters-matrix;
      name = "chatters-matrix";
    };
  in {
    packages.${system} = {
      inherit chatters-local chatters-signal chatters-matrix;
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

        # for notifications
        pkgs.libnotify
      ];

      PROTOC = "${pkgs.protobuf}/bin/protoc";
    };
  };
}
