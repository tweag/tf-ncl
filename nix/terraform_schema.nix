providers:
{ lib, runCommand, formats, terraform, cacert }:
let
  required_providers = lib.mapAttrs
    (_: p: {
      inherit (p) version;
      source = p.provider-source-address;
    })
    providers;

  providersJson = (formats.json { }).generate "providers.json" required_providers;

  mainJson = (formats.json { }).generate "main.tf.json" {
    terraform.required_providers = required_providers;
  };

  terraform-with-plugins = terraform.withPlugins (_: lib.attrValues providers);
in
runCommand "schema"
{
  passthru = {
    inherit providers;
  };
} ''
  cp ${mainJson} main.tf.json
  ${terraform-with-plugins}/bin/terraform init

  mkdir -p $out
  ${terraform-with-plugins}/bin/terraform providers schema -json >$out/schema.json
  cp ${providersJson} $out/providers.json
''
