{ lib, runCommand, tf-ncl, jsonSchema }:
assert lib.length (lib.attrNames jsonSchema.providers) == 1;
let
  provider-name = lib.head (lib.attrNames jsonSchema.providers);
  provider = jsonSchema.providers.${provider-name};
in
runCommand "${provider-name}-schema.ncl"
{ } ''
  ${tf-ncl}/bin/tf-ncl ${provider-name} ${provider.version} <${jsonSchema} > $out
''
