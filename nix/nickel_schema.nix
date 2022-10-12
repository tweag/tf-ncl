{ lib, runCommand, tf-ncl, jsonSchema }:
runCommand "${jsonSchema.provider.name}-schema.ncl"
{ } ''
  ${tf-ncl}/bin/tf-ncl ${jsonSchema.provider.name} ${jsonSchema.provider.version} <${jsonSchema} > $out
''
