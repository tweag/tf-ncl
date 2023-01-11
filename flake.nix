{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    utils.url = github:numtide/flake-utils;
    nickel.url = github:tweag/nickel;
    import-cargo.url = github:edolstra/import-cargo;
    rust-overlay = {
      url = github:oxalica/rust-overlay;
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    crane = {
      url = github:ipetkov/crane;
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pre-commit-hooks = {
      url = github:cachix/pre-commit-hooks.nix;
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
  };
  nixConfig = {
    extra-substituters = [ "https://tweag-nickel.cachix.org" ];
    extra-trusted-public-keys = [ "tweag-nickel.cachix.org-1:GIthuiK4LRgnW64ALYEoioVUQBWs0jexyoYVeLDBwRA=" ];
  };
  outputs = { self, utils, ... }@inputs:
    utils.lib.eachSystem (with utils.lib.system; [ x86_64-linux aarch64-linux ]) (system:
      let
        lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
        version = builtins.substring 0 8 lastModifiedDate;

        pkgs = import inputs.nixpkgs {
          localSystem = { inherit system; };
          config = { };
          overlays = [
            (import inputs.rust-overlay)
          ];
        };

        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.stable.latest.minimal.override {
          extensions = [
            "rust-src"
            "rust-analysis"
            "rustfmt-preview"
            "clippy"
          ];
          targets = [ (pkgs.rust.toRustTarget pkgs.stdenv.hostPlatform) ];
        };

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

        tf-ncl-src = craneLib.cleanCargoSource ./.;

        craneArgs = (craneLib.crateNameFromCargoToml { cargoToml = ./tf-ncl/Cargo.toml; }) // {
          src = tf-ncl-src;
        };

        cargoArtifacts = craneLib.buildDepsOnly craneArgs;

        tf-ncl = craneLib.buildPackage (craneArgs // {
          inherit cargoArtifacts;
        });

        schema-fetch = pkgs.buildGoModule {
          pname = "schema-fetch";
          inherit version;

          src = ./schema-fetch;

          vendorSha256 = null;
        };

        pre-commit = inputs.pre-commit-hooks.lib.${system}.run {
          src = ./.;
          tools = {
            inherit (pkgs) cargo rustfmt;
          };
          hooks = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
          };
        };

        terraformProviders = removeAttrs pkgs.terraform-providers.actualProviders [
          "checkpoint" # build is broken
        ];
      in
      rec {
        checks =
          schemas //
          (lib.mapAttrs'
            (name: drv: lib.nameValuePair "check-${name}" (
              let
                conf = pkgs.writeText "main.tf.ncl" ''
                  let Tf = import "${drv}" in
                  ({
                    config.output = {
                      "ip".value = "1.2.3.4",
                    }
                  } | Tf.Config).renderable_config
                '';
              in
              pkgs.runCommand "check-${name}" { } ''
                ${inputs.nickel.packages.${system}.default}/bin/nickel export -f ${conf} > $out
              ''
            ))
            schemas) //
          {
            inherit tf-ncl pre-commit;
          };

        packages = {
          default = packages.tf-ncl;
          inherit tf-ncl schema-fetch;
          terraform = pkgs.terraform;
        };

        inherit terraformProviders;

        generateJsonSchema = providerFn: pkgs.callPackage
          (import "${self}/nix/terraform_schema.nix" (providerFn terraformProviders))
          { };

        generateSchema = providerFn: pkgs.callPackage
          "${self}/nix/nickel_schema.nix"
          { jsonSchema = generateJsonSchema providerFn; inherit (packages) tf-ncl; };

        schemas = lib.mapAttrs
          (name: p: generateSchema (_: { ${name} = p; }))
          terraformProviders;

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks;
          buildInputs = with pkgs; [
            cargo
            rustc
            terraform
            inputs.nickel.packages.${system}.default
            rust-analyzer
            rustfmt
            clippy
            nixpkgs-fmt

            go
            gopls
            gotools
            go-tools
            gofumpt
          ];
          shellHook = ''
            ${pre-commit.shellHook}
          '';
        };
      });
}
