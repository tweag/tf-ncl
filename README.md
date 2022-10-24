# Terraform Configurations With Nickel

This repository contains a tool `tf-ncl` for generating [Nickel](https://github.com/tweag/nickel) contracts out of [terraform](https://www.terraform.io) provider schemas.

# How?
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

## How to use the result?
Excerpted from [tf-ncl-examples](https://github.com/tweag/tf-ncl-examples):
```
let Tf = import "./schema.ncl" in
let cfg = {
  provider.libvirt = [
    { uri = "qemu:///session" },
  ],

  resource."libvirt_network"."example" = {
    name = "example",
    mode = "nat",
    domain = "example.test",
    addresses = [ "10.17.3.0/24" ],
    dhcp = [{
      enabled = false,
    }],
    dns = [{
      enabled = true,
      local_only = false,
    }],
  },

  resource."libvirt_volume"."centos7-qcow2" = {
    name = "centos7.qcow2",
    pool = "default",
    source = "https://cloud.centos.org/centos/7/images/CentOS-7-x86_64-GenericCloud.qcow2",
    format = "qcow2",
  },

  resource."libvirt_domain"."centos7" = {
    name = "centos7",
    memory = 2048,
    vcpu = 2,
    network_interface = [{
      network_id = resource.libvirt_network.example.id,
      wait_for_lease = true,
      addresses = [ "10.17.3.2" ],
    }],
    disk = [{
      volume_id = resource.libvirt_volume.centos7-qcow2.id,
      scsi = true,
      file = "",
      block_device = "",
      url = "",
      wwn = "",
    }],
    console = [{
      type = "pty",
      target_type = "serial",
      target_port = "0",
    }],
  },
} | Tf.Configuration in
Tf.mkConfig cfg
```
