package main

import (
	// "encoding/json"

	"fmt"

	"github.com/davecgh/go-spew/spew"
	"github.com/hashicorp/go-version"
	tfaddr "github.com/hashicorp/terraform-registry-address"
	"github.com/hashicorp/terraform-schema/module"
	tfschema "github.com/hashicorp/terraform-schema/schema"
)

type Schema struct {
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
  schema, err := sm.SchemaForModule(&module.Meta{
    ProviderRequirements: provider_reqs,
  })
  if err != nil {
    panic(err)
  }

  fmt.Println(schema.BlockTypes())

  for key := range schema.Attributes {
    fmt.Println(key)
  }

  spew.Dump(schema.Blocks["provider"])
  // json,_ := json.Marshal(schema)
  // fmt.Println(string(json))
}
