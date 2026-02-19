{ lib, writeShellScriptBin, terraform, generateSchema, nickel, terraformProviders }:
{
  # A function returning an attrset mapping local names for providers to provider derivations from nixpkgs
  # It will be passed an attrset with all available providers from nixpkgs
  providers
, extraNickelInput ? ""
}:
let
  ncl-schema = generateSchema terraformProviders terraform providers;
  terraform-with-plugins = terraform.withPlugins (p: lib.attrValues (providers terraformProviders));
in
{
  terraform = terraform-with-plugins;

  link-schema = writeShellScriptBin "link-schema" ''
    set -e
    ln -sf ${ncl-schema} tf-ncl-schema.ncl
  '';

  run-nickel = writeShellScriptBin "run-nickel" ''
    set -e
    link-schema
    ${nickel}/bin/nickel export > main.tf.json <<EOF
      ((import "main.ncl") & {
        ${extraNickelInput}
      }).renderable_config
    EOF
  '';

  run-terraform = writeShellScriptBin "run-terraform" ''
    set -e
    run-nickel
    ${terraform-with-plugins}/bin/terraform "$@"
  '';
}
