{ provider, outputHash ? null }:
{ lib, runCommand, formats, terraform, cacert }:
let
  mainJson = (formats.json { }).generate "main.tf.json" (lib.filterAttrsRecursive (_:v: v != null) {
    terraform.required_providers.${provider.name} = {
      inherit (provider) source version;
    };
  });
in
runCommand "${provider.name}-schema.json"
{
  SSL_CERT_FILE = "${cacert.out}/etc/ssl/certs/ca-bundle.crt";
  outputHashMode = "flat";
  outputHashAlgo = "sha256";
  inherit outputHash;
  passthru = {
    inherit provider;
  };
} ''
  cp ${mainJson} main.tf.json
  ${terraform}/bin/terraform init
  ${terraform}/bin/terraform providers schema -json >$out
''
