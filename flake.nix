{
  description = "tuicr - Terminal UI for Code Reviews";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    eachSystem = nixpkgs.lib.genAttrs systems;
    pkgsFor = eachSystem (system: import nixpkgs {
      inherit system;
      overlays = [ self.overlays.default ];
    });
  in {
    overlays.default = final: prev: {
      tuicr = final.rustPlatform.buildRustPackage {
        pname = "tuicr";
        version = "${(builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version}-${self.shortRev or self.dirtyShortRev or "dirty"}";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;
        # Tests require git repo access and network, unavailable in Nix sandbox
        doCheck = false;
        nativeBuildInputs = [ final.pkg-config ];
        buildInputs = with final; [ openssl ]
          ++ final.lib.optionals final.stdenv.isDarwin [
            darwin.apple_sdk.frameworks.AppKit    # arboard (clipboard)
            darwin.apple_sdk.frameworks.Security  # libgit2 (TLS)
          ];
        meta.mainProgram = "tuicr";
      };
    };

    packages = eachSystem (system: {
      tuicr = pkgsFor.${system}.tuicr;
      default = self.packages.${system}.tuicr;
    });

    devShells = eachSystem (system: {
      default = pkgsFor.${system}.mkShell {
        inputsFrom = [ self.packages.${system}.tuicr ];
        packages = [ pkgsFor.${system}.rust-analyzer ];
      };
    });

    formatter = eachSystem (system: pkgsFor.${system}.nixfmt);
  };
}
