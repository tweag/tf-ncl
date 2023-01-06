package main

import (
	"encoding/json"

	"fmt"

	"github.com/davecgh/go-spew/spew"
	"github.com/hashicorp/go-version"
	"github.com/hashicorp/hcl-lang/schema"
	tfaddr "github.com/hashicorp/terraform-registry-address"
	"github.com/hashicorp/terraform-schema/module"
	tfschema "github.com/hashicorp/terraform-schema/schema"
)

func convert_attribute(as *schema.AttributeSchema) Attribute {
  attr := Attribute{
    Description: as.Description.Value,
    Optional: as.IsOptional,
  }
  return attr
}

func convert_block(bs *schema.BlockSchema) Attribute {
  spew.Dump(bs.DependentBody)
  attr := Attribute{
    Description: bs.Description.Value,
    Optional: true,
  }
  if bs.Body != nil {
    attr.Type = Dynamic{} //assemble_body(bs.Body)
  }
  return attr
}

func assemble_body(bs *schema.BodySchema) map[string]Attribute {
  schema := make(map[string]Attribute)
  for key, attr := range bs.Attributes {
    schema[key] = convert_attribute(attr)
  }

  for key, block := range bs.Blocks {
    schema[key] = convert_block(block)
  }

  return schema
}

func main() {
  coreSchema, err := tfschema.CoreModuleSchemaForVersion(tfschema.LatestAvailableVersion)
  if err != nil {
    panic(err)
  }
  sm := tfschema.NewSchemaMerger(coreSchema)
  provider_reqs := make(map[tfaddr.Provider]version.Constraints)

  c, _ := version.NewConstraint("0.7.0")
  provider_reqs[tfaddr.NewProvider("registry.terraform.io", "dmacvicar", "libvirt")] = c
  tf_schema, err := sm.SchemaForModule(&module.Meta{
    ProviderRequirements: provider_reqs,
  })
  if err != nil {
    panic(err)
  }

  json,_ := json.Marshal(assemble_body(tf_schema))
  fmt.Println(string(json))
}
