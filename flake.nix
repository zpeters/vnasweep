{
  description = "Capture, name, and overlay NanoVNA sweeps — Tauri desktop app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        # Rust/Cargo comes from rustup, not this shell. This just
        # provides the surrounding dev tooling (see justfile).
        #
        # mkShellNoCC (not mkShell): a plain `mkShell` pulls in a C
        # stdenv and exports NIX_LDFLAGS with `-L .../nix/store/...libiconv`,
        # which makes a rustup `cargo build --release` link libiconv from
        # the Nix store. The resulting .app then crashes at launch off
        # this machine ("Library not loaded: .../libiconv.2.dylib").
        devShells.default = pkgs.mkShellNoCC {
          packages = [
            pkgs.just
            pkgs.nodejs
            pkgs.resvg # renders assets/logo/*.svg (just logo)
          ];
        };
      }
    );
}
