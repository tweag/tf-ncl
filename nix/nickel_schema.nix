{ lib, runCommand, tf-ncl, jsonSchema }:
runCommand "schema.ncl"
{ } ''
  ${tf-ncl}/bin/tf-ncl ${jsonSchema}/providers.json ${jsonSchema}/schema.json > $out
''
