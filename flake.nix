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

        tf-ncl-src = pkgs.lib.cleanSourceWith {
          src = pkgs.lib.cleanSource ./.;
          filter = path: type: builtins.any (filter: filter path type) [
            (path: _type: builtins.match ".*\.ncl$" path != null)
            craneLib.filterCargoSources
          ];
        };

        craneArgs = (craneLib.crateNameFromCargoToml { cargoToml = ./tf-ncl/Cargo.toml; }) // {
          src = tf-ncl-src;
        };

        cargoArtifacts = craneLib.buildDepsOnly craneArgs;

        tf-ncl = craneLib.buildPackage (craneArgs // {
          inherit cargoArtifacts;
        });

        schema-merge = pkgs.buildGoModule {
          pname = "schema-merge";
          inherit version;

          src = ./schema-merge;

          vendorHash = "sha256-CtWf4H/TdxLQEdqjjybd5V8HGerC4VQQRyGilWkcmeY=";
        };

        pre-commit = inputs.pre-commit-hooks.lib.${system}.run {
          src = ./.;
          tools = {
            inherit (pkgs) cargo rustfmt;
          };
          hooks = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
            gofmt.enable = true;
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
                    config = {
                      output = {
                        "ip".value = "1.2.3.4",
                      },
                      variable."test-var" = {
                        sensitive = true,
                      },
                      terraform.backend.local = {
                        path = "dummy path"
                      },
                    },
                  } | Tf.Config).renderable_config
                '';
              in
              pkgs.runCommand "check-${name}" { } ''
                ${inputs.nickel.packages.${system}.default}/bin/nickel export -f ${conf} > $out
              ''
            ))
            schemas) //
          {
            inherit tf-ncl schema-merge pre-commit;
          };

        packages = {
          default = packages.tf-ncl;
          inherit tf-ncl schema-merge;
          terraform = pkgs.terraform;
        };

        inherit terraformProviders;

        generateJsonSchema = providerFn: pkgs.callPackage
          (import "${self}/nix/terraform_schema.nix" (providerFn terraformProviders))
          { inherit (packages) schema-merge; };

        generateSchema = providerFn: pkgs.callPackage
          "${self}/nix/nickel_schema.nix"
          { jsonSchema = generateJsonSchema providerFn; inherit (packages) tf-ncl; };

        schemas = lib.mapAttrs
          (name: p: generateSchema (_: { ${name} = p; }))
          terraformProviders;

        templates = rec {
          hello-tf = {
            path = ./examples/hello-tf;
            description = ''
              A minimal Nix flake containing a development shell for terraform with only the `null` provider.
            '';
          };

          default = hello-tf;
        };

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
