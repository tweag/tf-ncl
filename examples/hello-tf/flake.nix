{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    nickel.url = github:tweag/nickel;
    tf-ncl.url = github:tweag/tf-ncl;
    utils.url = github:numtide/flake-utils;
  };

  outputs = { self, utils, ... }@inputs: utils.lib.eachDefaultSystem (system:
    let

      # Make a nixpkgs package set for pulling Terraform and providers from
      pkgs = import inputs.nixpkgs {
        localSystem = { inherit system; };
        config = { };
        overlays = [ ];
      };

      # Declare the Terraform providers to use. This is a function that gets
      # passed an attribute set of all providers known to nixpkgs.
      providers = p: { inherit (p) null; };

      # The terraform.withPlugins function from nixpkgs takes a list of
      # providers instead of an attribute set. Hence the `attrValues` dance.
      terraform-with-plugins = pkgs.terraform.withPlugins
        (p: pkgs.lib.attrValues (providers p));

      inherit (inputs.nickel.packages.${system}) nickel lsp-nls;

      ncl-schema = inputs.tf-ncl.generateSchema.${system} providers;

      # This is a wrapper script for Terraform that automatically symlinks the
      # generated Nickel schema into the current working directory and
      # regenerates the Terraform JSON from the Nickel configuration. When in
      # the development shell, this wrapper replaces the `terraform` CLI tool.
      # Outside of it, it can be called with `nix run .#terraform`.
      run-terraform = pkgs.writeShellScriptBin "terraform" ''
        set -e
        ln -sf ${ncl-schema} schema.ncl
        ${nickel}/bin/nickel export > main.tf.json <<EOF
          (import "./main.tf.ncl").renderable_config
        EOF
        ${terraform-with-plugins}/bin/terraform "$@"
      '';
    in
    {
      packages = {
        inherit ncl-schema;
        terraform = run-terraform;
        default = ncl-schema;
      };
      apps = rec {
        terraform = utils.lib.mkApp { drv = run-terraform; };
        default = terraform;
      };
      devShell = pkgs.mkShell {
        buildInputs = [
          lsp-nls
          nickel
          run-terraform
        ];
      };
    });
}
