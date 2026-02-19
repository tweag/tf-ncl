# Terraform Configurations With Nickel

This repository contains tooling for generating
[Nickel](https://github.com/tweag/nickel) contracts out of
[Terraform](https://www.terraform.io) provider schemas.

It enables configurations to be checked against provider specific contracts
before calling Terraform to perform the deployment. Nickel can natively generate
outputs as JSON, YAML or TOML. Since Terraform can accept its deployment
configuration as JSON, you can straightforwardly export a Nickel configuration,
adhering to the right format, to Terraform. Tf-Ncl provides a framework for
ensuring a Nickel configuration has this specific format. Specifically, Tf-Ncl
is a tool to generate Nickel contracts that describe the configuration schema
expected by a set of Terraform providers.

## Starting a Tf-Ncl configuration

The easiest way to get started is to use the `hello-tf` flake template:

```console
nix flake init -t github:tweag/tf-ncl#hello-tf
```

This will leave you with a `flake.nix` file containing some glue code for
getting a Nickel contract out of `tf-ncl`, evaluating a Nickel configuration
and calling Terraform. It's as easy as

```console
nix develop -c run-terraform init
nix develop -c run-terraform apply
```

### Using `mkTfNcl` in Your Own Flake

The `mkTfNcl` function creates a Terraform wrapper that:
- Automatically generates Nickel contracts from provider schemas
- Uses [prj-spec](https://github.com/numtide/prj-spec) to manage data directories
- Exports Nickel configurations as Terraform JSON

**Why prj-spec?**

Terraform provider schemas grow with each plugin. If you have 10 deployment tasks with different providers, the combined schema becomes huge, causing Nickel export to slow down significantly. With prj-spec:

- **Schema modularization**: Each task gets its own schema at `$PRJ_DATA_DIR/tf-ncl/<name>/schema.ncl`
- **Faster exports**: Export only the schema needed for the specific task
- **Temp file management**: Automatically manages generated `.tf.json` files

**Why terraform-backend-git?**

For GitOps workflows, storing Terraform state in Git provides:
- **State versioning**: Track state changes with Git history
- **Collaboration**: Share state via Git branches/PRs
- **Audit trail**: Full history of infrastructure changes

Default data path: `$PRJ_DATA_DIR/tf-ncl/<name>/`

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `name` | string | Project name (used for data directory) |
| `nixpkgs` | Nixpkgs | Nixpkgs instance |
| `providers` | function | Provider selector, e.g. `p: { inherit (p) aws null; }` |
| `nickel` | package | Nickel binary (optional, defaults to nixpkgs.nickel) |
| `terraform` | package | Terraform binary (optional, defaults to nixpkgs.opentofu) |
| `extraNickelInput` | string | Extra Nickel input (optional) |
| `terraform-backend-git` | attrs | Git backend config for GitOps (optional) |

**Example in your flake:**

```nix
{
  inputs = {
    tf-ncl.url = "github:tweag/tf-ncl";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs: inputs.utils.lib.eachDefaultSystem (system:
    let
      pkgs = import inputs.nixpkgs { inherit system; };
    in
    {
      apps = {
        default = inputs.tf-ncl.lib.${system}.mkTfNcl {
          name = "my-project";
          nixpkgs = pkgs;
          providers = p: {
            inherit (pkgs.terraform-providers) null aws;
          };
        };
      };
    });
}
```

**Usage:**

```console
# The wrapper expects: <ncl-file> <terraform-command> [args...]
nix run .#my-project main.ncl init
nix run .#my-project main.ncl plan
nix run .#my-project main.ncl apply
```

The wrapper will:
1. Link schema to `$PRJ_DATA_DIR/tf-ncl/my-project/schema.ncl`
2. Export Nickel config to `$PRJ_DATA_DIR/tf-ncl/my-project/main.tf.json`
3. Run Terraform in that directory

**Access devshell:**

The wrapper exposes `passthru.devshell` for entering a development shell with all required tools:

```nix
# In your flake outputs:
devShells.default = inputs.tf-ncl.lib.${system}.mkTfNcl {
  name = "my-project";
  nixpkgs = pkgs;
  providers = p: { inherit (pkgs.terraform-providers) null; };
}.passthru.devshell
```

Without Nix it's a bit more complicated. You will need to obtain the Nickel
contract using the tools in this repository. Take a look at [the working
principle](#how) for an overview of the process. The most involved step will be
calling `schema-merge` with extracted Terraform provider schemas, see [the nix
code](nix/terraform_schema.nix) for inspiration.

Once you have a project set up, you can start writing Nickel configuration
code. To quote from the `hello-tf` example:

```nickel
let Tf = import "./schema.ncl" in
{
  config.resource.null_resource.hello-world = {
    provisioner.local-exec = [{
      command = m%"
        echo 'Hello, world!'
      "%
    }],
  },
} | Tf.Config
```

Anything goes! You just need to ensure that your Terraform configuration ends
up in the toplevel attribute `config` and that your entire configuration
evaluates to a record satisfying the `Tf.Config` contract.

To actually turn the Nickel code into a JSON configuration file understood by
Terraform, you need to call `nickel` to export the `renderable_config` toplevel
attribute introduced by the `Tf.Config` contract:

```console
nickel export <your-toplevel-file.ncl> --field renderable_config
```

This can be useful for inspecting the result of your code. But usually it will
be easier to use the wrapper script for Terraform provided in the [`hello-tf`
flake template](examples/hello-tf/flake.nix).

For inspiration on what's possible with Nickel, take a look at [the
examples](examples/). Happy hacking!

## How

Unfortunately, Terraform doesn't expose an interface for extracting a machine
readable specification for the provider independent configuration it supports.
Because of that this repository contains two tools and some glue written in
Nix. Maybe this flowchart helps:

```mermaid
flowchart LR
    subgraph Nix
        direction TB
        providerSpec(Required Providers);
        providerSchemasNix(Terraform provider schemas);
        providerSpec -- generateJsonSchema --> providerSchemasNix;
    end
    
    subgraph schema-merge
        direction TB
        providerSchemasGo(Terraform provider schemas);
        merged-json-go(Merged JSON w/ Terraform builtins);
        providerSchemasGo --> merged-json-go;
    end

    subgraph tf-ncl
        direction TB
        merged-json-rust(Merged JSON w/ Terraform builtins);
        nickel-contracts(Monolithic Nickel contract)
        merged-json-rust --> nickel-contracts;
    end

    Nix --> schema-merge
    schema-merge --> tf-ncl
```

The entire process is packaged up in a Nix function `generateSchema` which is
exposed as a flake output. Also, to generate a Nickel contract for a single
provider, there is a flake output `schemas`:

```console
nix build github:tweag/tf-ncl#schemas.aws
```

All providers available in `nixpkgs` are supported. The `generateSchema`
function can also be called manually. For example, to get a monolithic Nickel
schema for the `aws`, `github` and `external` Terraform providers, you could
use

```console
nix build --impure --expr \
  '(builtins.getFlake "github:tweag/tf-ncl).generateSchema.${builtins.currentSystem} (p: { inherit (p) aws github external; })'
```

## Status

This project is in active development and breaking changes should be expected.

- [x] Automatic contracts for Terraform provider schemas
- [x] Contracts for Terraform state backends [#14][i14], [#15][i15]
- [ ] More documentation [#13][i13]
- [ ] Natural handling of field references [#12][i12]

[i12]: https://github.com/tweag/tf-ncl/issues/12
[i13]: https://github.com/tweag/tf-ncl/issues/13
[i14]: https://github.com/tweag/tf-ncl/issues/14
[i15]: https://github.com/tweag/tf-ncl/issues/15

