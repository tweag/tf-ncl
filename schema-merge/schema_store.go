package main

import (
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"path"

	version "github.com/hashicorp/go-version"
	tfjson "github.com/hashicorp/terraform-json"
	tfaddr "github.com/hashicorp/terraform-registry-address"
	tfmodule "github.com/hashicorp/terraform-schema/module"
	tfschema "github.com/hashicorp/terraform-schema/schema"
)

type ProviderSpec struct {
	Source  string `json:"source"`
	Version string `json:"version"`
}

type Provider struct {
	Version      *version.Version
	SchemaReader io.Reader
}

func NewProvider(spec ProviderSpec, reader io.Reader) (*Provider, error) {
	version, e := version.NewVersion(spec.Version)
	if e != nil {
		return nil, e
	}

	return &Provider{
		Version:      version,
		SchemaReader: reader,
	}, nil
}

type SchemaStore struct {
	local_names map[string]tfaddr.Provider
	providers   map[tfaddr.Provider]Provider
}

func NewSchemaStore(fsys fs.FS) (*SchemaStore, error) {
	specs_bytes, e := fs.ReadFile(fsys, "providers.json")
	if e != nil {
		return nil, e
	}

	var specs map[string]ProviderSpec
	e = json.Unmarshal(specs_bytes, &specs)
	if e != nil {
		return nil, e
	}

	providers := map[tfaddr.Provider]Provider{}
	local_names := map[string]tfaddr.Provider{}
	for name, spec := range specs {
		k, e := tfaddr.ParseProviderSource(spec.Source)
		if e != nil {
			return nil, e
		}
		local_names[name] = k

		reader, e := fsys.Open(path.Join("schemas", fmt.Sprintf("%s.json", name)))
		if e != nil {
			return nil, e
		}

		p, e := NewProvider(spec, reader)
		if e != nil {
			return nil, e
		}

		providers[k] = *p
	}
	return &SchemaStore{
		local_names: local_names,
		providers:   providers,
	}, nil
}

func (s *SchemaStore) ProviderReqs() map[tfaddr.Provider]version.Constraints {
	ret := map[tfaddr.Provider]version.Constraints{}
	for k, v := range s.providers {
		ret[k] = version.MustConstraints(version.NewConstraint(fmt.Sprintf("=%s", v.Version)))
	}
	return ret
}

func (s *SchemaStore) ProviderRefs() map[tfmodule.ProviderRef]tfaddr.Provider {
	ret := map[tfmodule.ProviderRef]tfaddr.Provider{}
	for k, v := range s.local_names {
		ret[tfmodule.ProviderRef{LocalName: k}] = v
	}
	return ret
}

func (s *SchemaStore) ProviderSchema(modPath string, addr tfaddr.Provider, vc version.Constraints) (*tfschema.ProviderSchema, error) {
	version := s.providers[addr].Version
	if !vc.Check(version) {
		// the terraform-schema SchemaMerger doesn't actually care if we return an error
		panic("Incompatible provider version requested")
	}

	var jsonSchemas tfjson.ProviderSchemas
	e := json.NewDecoder(s.providers[addr].SchemaReader).Decode(&jsonSchemas)
	if e != nil {
		// the terraform-schema SchemaMerger doesn't actually care if we return an error
		panic(e)
	}

	ps, ok := jsonSchemas.Schemas[addr.String()]
	if !ok {
		// the terraform-schema SchemaMerger doesn't actually care if we return an error
		panic(fmt.Errorf("%q: schema not found", addr))
	}

	schema := tfschema.ProviderSchemaFromJson(ps, addr)
	return schema, nil
}
