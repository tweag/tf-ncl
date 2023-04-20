{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    nickel.url = "github:tweag/nickel";
    topiary.url = "github:tweag/topiary";
    import-cargo.url = "github:edolstra/import-cargo";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
  };
  nixConfig = {
    extra-substituters = [ "https://tweag-nickel.cachix.org" ];
    extra-trusted-public-keys = [ "tweag-nickel.cachix.org-1:GIthuiK4LRgnW64ALYEoioVUQBWs0jexyoYVeLDBwRA=" ];
  };
  outputs = { self, utils, ... }@inputs:
    utils.lib.eachDefaultSystem
      (system:
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

            vendorHash = "sha256-HQQQlOqjI4EAB1JNhFmjt9SQrF2wOh6C7hrR8So0dxs=";
          };

          eval-github-yaml = pkgs.writeShellScript "eval-github-yaml" ''
            set -e
            find .github -name '*.ncl' -print0 | while IFS= read -d $'\0' f; do
              (echo "# DO NOT EDIT! Generated by eval-github-yaml from $f"; \
                ${inputs.nickel.packages.${system}.default}/bin/nickel export --format json -f "$f") > "''${f%.ncl}.yml"
            done
          '';

          pre-commit = inputs.pre-commit-hooks.lib.${system}.run {
            src = ./.;
            tools = {
              inherit (pkgs) cargo rustfmt;
            };
            hooks = {
              nixpkgs-fmt.enable = true;
              rustfmt.enable = true;
              gofmt.enable = true;
              github-ncl = {
                enable = true;
                name = "github-ncl";
                description = "TODO";
                files = "^(.github/.*\\.ncl|ncl/github/.*\\.ncl)$";
                entry = "${eval-github-yaml}";
              };
            };
          };

          terraformProviders = pkgs.terraform-providers.actualProviders;

          release = pkgs.runCommand "release-tarball"
            {
              nativeBuildInputs = [ pkgs.pixz ];
            } ''
            mkdir -p $out
            mkdir tf-ncl
            ${lib.concatLines (lib.flip lib.mapAttrsToList inputs.self.schemas.${system} (provider: schema: ''
              cp ${schema} tf-ncl/${provider}.ncl
            ''))}
            tar --sort=name --mtime='@1' --owner=0 --group=0 --numeric-owner -c tf-ncl/*.ncl | pixz -t > $out/tf-ncl.tar.xz
          '';

          test-single-example = template: pkgs.writeShellScript "test-${template}" ''
            set -e -x
            temp_directory=$(mktemp -d)
            trap 'rm -r -- "$temp_directory"' EXIT

            cd "$temp_directory"
            nix flake init -t "${self}#github-users"
            nix develop --override-input tf-ncl "${self}" -c run-nickel
          '';

          test-examples = pkgs.writeShellScriptBin "test-examples" ''
            set -e -x
            ${lib.concatMapStringsSep "\n" (tmpl: "${test-single-example tmpl}") (lib.attrNames self.templates)}
          '';
        in
        {
          checks =
            self.schemas.${system} //
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
              self.schemas.${system}) //
            {
              inherit tf-ncl schema-merge pre-commit;
            };

          packages = {
            default = tf-ncl;
            terraform = pkgs.terraform;
            inherit tf-ncl schema-merge release test-examples;
          } // lib.mapAttrs' (name: value: lib.nameValuePair "schema-${name}" value) self.schemas.${system};

          inherit terraformProviders;

          generateJsonSchema = providerFn: pkgs.callPackage
            (import ./nix/terraform_schema.nix (providerFn terraformProviders))
            { inherit (self.packages.${system}) schema-merge; };

          generateSchema = providerFn: pkgs.callPackage
            ./nix/nickel_schema.nix
            { jsonSchema = self.generateJsonSchema.${system} providerFn; inherit (self.packages.${system}) tf-ncl; };

          schemas = lib.mapAttrs
            (name: p: self.generateSchema.${system} (_: { ${name} = p; }))
            terraformProviders;

          lib = {
            mkDevShell = args:
              pkgs.mkShell {
                buildInputs = lib.attrValues
                  (pkgs.callPackage ./nix/devshell.nix
                    {
                      generateSchema = self.generateSchema.${system};
                      inherit (inputs.nickel.packages.${system}) nickel;
                    }
                    args) ++ [
                  inputs.nickel.packages.${system}.nickel
                  inputs.nickel.packages.${system}.lsp-nls
                  inputs.topiary.packages.${system}.default
                ];
              };
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
              inputs.topiary.packages.${system}.default

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
        }) // {
      templates = rec {
        hello-tf = {
          path = ./examples/hello-tf;
          description = ''
            A minimal Nix flake containing a development shell for terraform with only the `null` provider.
          '';
        };

        github-users = {
          path = ./examples/github-users;
          description = ''
            A toy example demonstrating how to transform a custom configuration schema into Terraform compatible resource specifications.
          '';
        };

        default = hello-tf;
      };
    };
}
