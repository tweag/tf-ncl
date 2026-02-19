providers:
{ lib, runCommand, formats, terraform, cacert, schema-merge }:
let
  cleanName = name: builtins.replaceStrings [ "_" ] [ "-" ] name;

  # Get source address for a provider
  getProviderSource = p:
    lib.toLower
      (p.override (
        oldArgs:
        if (builtins.hasAttr "homepage" oldArgs) && (terraform.pname == "opentofu") then
          {
            provider-source-address =
              lib.replaceStrings
                [ "https://registry.terraform.io/providers" ]
                [
                  "registry.opentofu.org"
                ]
                oldArgs.homepage;
          }
        else
          { }
      )).provider-source-address;

  required_providers = providers:
    lib.mapAttrs
      (name: p: {
        inherit (p) version;
        source = getProviderSource p;
      })
      providers;

  retrieveProviderSchema = name: provider:
    let
      # Use only clean name for OpenTofu
      mainJson = (formats.json { }).generate "main.tf.json" {
        terraform.required_providers = {
          "${cleanName name}" = {
            source = getProviderSource provider;
            version = provider.version;
          };
        };
      };

      terraform-with-plugins = terraform.withPlugins (_: [ provider ]);
    in
    runCommand "${cleanName name}.json" { } ''
      cp ${mainJson} main.tf.json
      ${lib.getExe terraform-with-plugins} init
      ${lib.getExe terraform-with-plugins} providers schema -json >$out
    '';

  # Single provider's required_providers entry
  providerSource = name: p: {
    version = p.version;
    source = getProviderSource p;
  };

  providersJson = (formats.json { }).generate "providers.json"
    (lib.mapAttrs' (name: p: lib.nameValuePair (cleanName name) (providerSource name p)) providers);
in
runCommand "schemas" { } ''
  mkdir schemas
  ${lib.concatStringsSep "\n" (lib.mapAttrsToList
    (name: provider: ''
      ln -s ${retrieveProviderSchema name provider} schemas/"${cleanName name}.json"
    '')
    providers)}
  ln -s ${providersJson} providers.json

  mkdir -p $out
  ln -s ${providersJson} $out/providers.json
  ${schema-merge}/bin/schema-merge . > $out/schema.json
''
