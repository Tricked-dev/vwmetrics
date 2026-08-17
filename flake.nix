{
  description = "Vaultwarden Prometheus metrics exporter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        inherit (pkgs) lib;
        manifest = lib.importTOML ./Cargo.toml;
        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          pname = manifest.package.name;
          version = manifest.package.version;
          strictDeps = true;
          nativeBuildInputs = [ pkgs.pkg-config ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        vwmetrics = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;

            meta = {
              inherit (manifest.package) description;
              license = lib.licenses.asl20;
              mainProgram = "vwmetrics";
            };
          }
        );

        app = {
          type = "app";
          program = lib.getExe vwmetrics;
        };
      in
      {
        packages = {
          default = vwmetrics;
          inherit vwmetrics;
        };

        apps = {
          default = app;
          vwmetrics = app;
        };

        checks.default = vwmetrics;

        devShells.default = craneLib.devShell {
          inputsFrom = [ vwmetrics ];
          packages = [ pkgs.nixfmt-rfc-style ];
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
