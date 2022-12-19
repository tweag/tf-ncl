# Terraform Configurations With Nickel

This repository contains a tool `tf-ncl` for generating [Nickel](https://github.com/tweag/nickel) contracts out of [terraform](https://www.terraform.io) provider schemas.

## How?
There is a collection of examples [here](https://github.com/tweag/tf-ncl-examples).

Get a Nickel contract for the terraform providers `libvirt`, `random` and `external` the quick and dirty way:
```
nix build --impure --expr '(builtins.getFlake (builtins.toString ./.)).generateSchema.${builtins.currentSystem} (p: { inherit(p) libvirt random external; })'
```

The `tf-ncl` tool can also be called directly. First you need a file `providers.json` specifying the providers you want to use with their chosen local names, e.g.:
```json
{
  "libvirt": {
    "version": "0.7.0",
    "source": "registry.terraform.io/dmacvicar/libvirt"
  }
}
```
Then you need to extract a schema from `terraform`. In a temporary directory, create a `main.tf` file containing only the `required_providers` stanza for `terraform`:
```
terraform {
  required_providers {
    libvirt = {
      source = "registry.terraform.io/dmacvicar/libvirt"
      version = "0.7.0"
    }
  }
}
```
Then run
```
terraform init
terraform providers schema -json > schema.json
```
Finally, generate the Nickel contracts with
```
tf-ncl providers.json schema.json
```

## Status

This project is in active development and breaking changes should be expected.

- [x] Automatic contracts for Terraform provider schemas
- [ ] More documentation [#13][i13]
- [ ] Natural handling of field references [#12][i12]
- [ ] Contracts for Terraform state backends [#14][i14], [#15][i15]

[i12]: https://github.com/tweag/tf-ncl/issues/12
[i13]: https://github.com/tweag/tf-ncl/issues/13
[i14]: https://github.com/tweag/tf-ncl/issues/14
[i15]: https://github.com/tweag/tf-ncl/issues/15

