package main

import (
	"errors"
	"fmt"

	"github.com/davecgh/go-spew/spew"
	version "github.com/hashicorp/go-version"
	tfaddr "github.com/hashicorp/terraform-registry-address"
	tfschema "github.com/hashicorp/terraform-schema/schema"
)

type ProviderSpec struct {
	Source  string `json:"source"`
	Version string `json:"version"`
}

type Provider struct {
  Version *version.Version
}

func NewProvider(spec ProviderSpec) (*Provider, error) {
  version, e := version.NewVersion(spec.Version)
  if e != nil {
    return nil, e
  }

  return &Provider{
    Version: version,
  }, nil
}

type SchemaStore struct {
  providers map[tfaddr.Provider]Provider
}

func NewSchemaStore(specs map[string]ProviderSpec) (*SchemaStore, error) {
  providers := map[tfaddr.Provider]Provider{}
  for _, spec := range specs {
    k, e := tfaddr.ParseProviderSource(spec.Source)
    if e != nil {
      return nil, e
    }

    p, e := NewProvider(spec)
    if e != nil {
      return nil, e
    }

    providers[k] = *p
  }
  return &SchemaStore{
  	providers: providers,
  }, nil
}

func (s *SchemaStore)ProviderReqs() map[tfaddr.Provider]version.Constraints {
  ret := map[tfaddr.Provider]version.Constraints{}
  for k, v := range s.providers {
    ret[k] = version.MustConstraints(version.NewConstraint(fmt.Sprintf("=%s", v.Version)))
  }
  return ret
}

func (s *SchemaStore) ProviderSchema(modPath string, addr tfaddr.Provider, vc version.Constraints) (*tfschema.ProviderSchema, error) {
  version := s.providers[addr].Version
  if !vc.Check(version) {
    panic("Incompatible provider version requested")
  }

  spew.Dump(addr)
  spew.Dump(s.providers[addr])
  return nil, errors.New("not implemented")
}
