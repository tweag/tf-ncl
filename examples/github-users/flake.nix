{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;
    nickel.url = github:tweag/nickel;
    tf-ncl.url = github:tweag/tf-ncl;
    utils.url = github:numtide/flake-utils;
  };

  nixConfig = {
    extra-substituters = [ "https://tweag-nickel.cachix.org" ];
    extra-trusted-public-keys = [ "tweag-nickel.cachix.org-1:GIthuiK4LRgnW64ALYEoioVUQBWs0jexyoYVeLDBwRA=" ];
  };

  outputs = { self, utils, ... }@inputs: utils.lib.eachDefaultSystem (system:
    let
      pkgs = import inputs.nixpkgs {
        localSystem = { inherit system; };
        config = { };
        overlays = [ ];
      };

      providers = p: {
        inherit (p) github null external;
      };

      terraform-with-plugins = inputs.tf-ncl.packages.${system}.terraform.withPlugins (p: pkgs.lib.attrValues (providers p));
      nickel = inputs.nickel.packages.${system}.default;

      run-terraform = pkgs.writeShellScriptBin "terraform" ''
        set -e
        if [[ "$#" -le 1 ]]; then
          echo "terraform <ncl-file> ..."
          exit 1
        fi

        ENTRY="''${1}"
        shift

        ln -sf ${self.packages.${system}.ncl-schema} schema.ncl
        ${nickel}/bin/nickel export > main.tf.json <<EOF
          (import "''${ENTRY}").renderable_config
        EOF
        ${terraform-with-plugins}/bin/terraform "$@"
      '';


    in
    rec {
      apps = {
        default = apps.terraform;
        terraform = utils.lib.mkApp { drv = run-terraform; };
      };

      packages = {
        default = packages.terraform;
        terraform = run-terraform;
        ncl-schema = inputs.tf-ncl.generateSchema.${system} providers;
      };

      devShell = pkgs.mkShell {
        buildInputs = [
          run-terraform
          nickel
        ];
      };
    });
}
