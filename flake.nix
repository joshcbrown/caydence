{
  description = "caydence devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    crane,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        craneLib = crane.lib.${system};

        crate = craneLib.buildPackage {
          src = craneLib.cleanCargoSource (craneLib.path ./.);
          strictDeps = true;

          buildInputs = with pkgs; [
            pkg-config
            libnotify
            glib
            gdk-pixbuf
            rust-bin.beta.latest.default
          ];
        };
      in
        with pkgs; {
          packages.default = crate;
          checks = {inherit crate;};
          devShells.default = mkShell {
            inputsFrom = [crate];

            shellHook = ''
              alias ls=eza
              alias find=fd
            '';
          };
        }
    );
}
