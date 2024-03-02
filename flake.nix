{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
  }: let
    overlays = [
      rust-overlay.overlays.default
      (final: prev: {
        rustToolchain = prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      })
    ];
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (system:
        f {
          pkgs = import nixpkgs {inherit overlays system;};
        });
  in {
    defaultPackage = forEachSupportedSystem ({pkgs}:
      pkgs.rustPlatform.buildRustPackage rec {
        pname = "dump";
        version = "0.1.0";
        src = ./.;
        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes = {
            "magika-0.1.0-dev" = "sha256-vXTKqdD/KuQABg13+/FJB5JlOVuWuD18z/kj2thCess=";
          };
        };
        env = {
          ORT_STRATEGY = "system";
          ORT_LIB_LOCATION = "${pkgs.onnxruntime}/lib/libonnxruntime.so";
          NIX_LDFLAGS = "-L${pkgs.onnxruntime}/lib";
        };
      });
    devShells = forEachSupportedSystem ({pkgs}: {
      default = pkgs.mkShell rec {
        packages = with pkgs; [
          rustToolchain
          openssl
          pkg-config
          rust-analyzer
          onnxruntime
          stdenv.cc.cc.lib
        ];
        # shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
        #     pkgs.alsaLib
        #     pkgs.udev
        #     ${stdenv.cc.cc.lib}/lib/
        #   ]}"'';
        LD_LIBRARY_PATH = nixpkgs.lib.makeLibraryPath packages;
      };
    });
  };
}
