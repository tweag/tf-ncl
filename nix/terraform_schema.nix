providers:
{ lib, runCommand, formats, terraform, cacert }:
let
  mainJson = (formats.json { }).generate "main.tf.json" (lib.filterAttrsRecursive (_:v: v != null) {
    terraform.required_providers = lib.mapAttrs
      (_: p: {
        inherit (p) version;
        source = p.provider-source-address;
      })
      providers;
  });

  terraform-with-plugins = terraform.withPlugins (_: lib.attrValues providers);
in
runCommand "schema.json"
{
  passthru = {
    inherit providers;
  };
} ''
  cp ${mainJson} main.tf.json
  ${terraform-with-plugins}/bin/terraform init
  ${terraform-with-plugins}/bin/terraform providers schema -json >$out
''
