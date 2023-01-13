package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"os"

	// "github.com/davecgh/go-spew/spew"
	"github.com/hashicorp/hcl-lang/lang"
	"github.com/hashicorp/hcl-lang/schema"
	"github.com/hashicorp/terraform-schema/module"
	tfschema "github.com/hashicorp/terraform-schema/schema"
	"github.com/zclconf/go-cty/cty"
)

func convert_primitive_type(t cty.Type) Type {
	switch {
	case t == cty.String:
		return Type{Tag: String}
	case t == cty.Number:
		return Type{Tag: Number}
	case t == cty.Bool:
		return Type{Tag: Bool}
	}
	return Type{Tag: Dynamic}
}

func has_object_type(ecs schema.ExprConstraints) (bool, *Type) {
	for _, ec := range ecs {
		switch c := ec.(type) {
		case schema.ObjectExpr:
			obj := map[string]Attribute{}
			for k, v := range c.Attributes {
				obj[k] = convert_attribute(v)
			}
			return true, &Type{
				Tag:    Object,
				Object: &obj,
			}
		}
	}
	return false, nil
}

func has_list_type(ecs schema.ExprConstraints) (bool, *Type) {
	for _, ec := range ecs {
		switch c := ec.(type) {
		case schema.SetExpr:
			content := extract_type(c.Elem)

			var min_items, max_items *uint64
			if c.MinItems != 0 {
				min_items = &c.MinItems
			}
			if c.MaxItems != 0 {
				max_items = &c.MaxItems
			}

			return true, &Type{
				Tag:      List,
				MinItems: min_items,
				MaxItems: max_items,
				Content:  &content,
			}
		case schema.ListExpr:
			content := extract_type(c.Elem)

			var min_items, max_items *uint64
			if c.MinItems != 0 {
				min_items = &c.MinItems
			}
			if c.MaxItems != 0 {
				max_items = &c.MaxItems
			}

			return true, &Type{
				Tag:      List,
				MinItems: min_items,
				MaxItems: max_items,
				Content:  &content,
			}
		}
	}
	return false, nil
}

// TODO(vkleen) Handle enumerations
func extract_type(ecs schema.ExprConstraints) Type {
	if is_object, t := has_object_type(ecs); is_object {
		return *t
	}

	if is_list, t := has_list_type(ecs); is_list {
		return *t
	}

	for _, ec := range ecs {
		switch c := ec.(type) {
		case schema.LiteralTypeExpr:
			switch {
			case c.Type.IsPrimitiveType():
				return convert_primitive_type(c.Type)
			}
			return Type{Tag: Dynamic}
		}
	}
	return Type{Tag: Dynamic}
}

func extract_optional_interpolation(as *schema.AttributeSchema) (bool, InterpolationStrategy) {
	switch {
	case as.IsOptional && !as.IsRequired && !as.IsComputed:
		return true, InterpolationStrategy{InterpolationType: Nickel}
	case !as.IsOptional && as.IsRequired && !as.IsComputed:
		return false, InterpolationStrategy{InterpolationType: Nickel}
	case !as.IsOptional && !as.IsRequired && as.IsComputed:
		// TODO(vkleen) Once interpolation of computed fields is properly handled,
		// these fields should no longer be optional
		return true, InterpolationStrategy{InterpolationType: Terraform, Force: true}
	case as.IsOptional && !as.IsRequired && as.IsComputed:
		// TODO(vkleen) Once interpolation of computed fields is properly handled,
		// these fields should no longer be optional
		return true, InterpolationStrategy{InterpolationType: Terraform, Force: false}
	}
	return true, InterpolationStrategy{InterpolationType: Nickel}
}

func convert_attribute(as *schema.AttributeSchema) Attribute {
	o, i := extract_optional_interpolation(as)
	attr := Attribute{
		Description:   as.Description.Value,
		Optional:      o,
		Interpolation: i,
		Type:          extract_type(as.Expr),
	}
	return attr
}

type DependentBody struct {
	key    schema.DependencyKeys
	schema *schema.BodySchema
}

func dependency_keys(bs *schema.BlockSchema) []DependentBody {
	ret := []DependentBody{}
	for sk, b := range bs.DependentBody {
		var dk schema.DependencyKeys
		json.Unmarshal([]byte(sk), &dk)
		ret = append(ret, DependentBody{
			key:    dk,
			schema: b,
		})
	}
	return ret
}

type Label struct {
	possible_values map[string]schema.BodySchema
	wildcard        bool
}

func classify_labels(bs *schema.BlockSchema) []Label {
	ret := []Label{}
	for i, l := range bs.Labels {
		if l.Completable {
			possible_values := map[string]schema.BodySchema{}
			for _, dk := range dependency_keys(bs) {
				for _, l := range dk.key.Labels {
					if l.Index == i {
						possible_values[l.Value] = *dk.schema
					}
				}
			}
			ret = append(ret, Label{possible_values: possible_values, wildcard: false})
		} else {
			ret = append(ret, Label{possible_values: map[string]schema.BodySchema{}, wildcard: true})
		}
	}
	return ret
}

var BLOCK_TYPE_MAP = map[string]schema.BlockType{
  "provider": schema.BlockTypeList,
  "moved": schema.BlockTypeList,
  "precondition": schema.BlockTypeList,
  "postcondition": schema.BlockTypeList,
}

func wrap_block_type(key string, bs *schema.BlockSchema, attr Attribute) Attribute {
  t := bs.Type
  if t == schema.BlockTypeNil {
    new_t, ok := BLOCK_TYPE_MAP[key]
    if ok {
      t = new_t
    } else {
      t = schema.BlockTypeObject
    }
  }
  switch t {
  case schema.BlockTypeObject:
    return attr
  case schema.BlockTypeList, schema.BlockTypeSet:
    return Attribute{
      Description: attr.Description,
      Optional: attr.Optional,
      Interpolation: attr.Interpolation,
      Type: Type{
        Tag: List,
        MinItems: &bs.MinItems,
        MaxItems: &bs.MaxItems,
        Content: &attr.Type,
      },
    }
  default:
    panic(errors.New(fmt.Sprint("Unknown block type ", t.GoString())))
  }
}

func assemble_blocks(key string, bs *schema.BlockSchema, labels []Label, accumulated_bodies []*schema.BodySchema) Attribute {
	if len(labels) == 0 {
		obj := assemble_bodies(accumulated_bodies...)
		description := bs.Description.Value
		if len(accumulated_bodies) > 0 {
			last_description := accumulated_bodies[len(accumulated_bodies)-1].Description
			if last_description.Kind != lang.NilKind {
				description = last_description.Value
			}
		}
		return wrap_block_type(key, bs, Attribute{
			Description: description,
			// TODO(vkleen): compute these values properly
			Optional:      true,
			Interpolation: InterpolationStrategy{InterpolationType: Nickel},
			Type: Type{
				Tag:    Object,
				Object: &obj,
			},
		})
	}
	l := labels[0]
	if l.wildcard {
		t := assemble_blocks(key, bs, labels[1:], accumulated_bodies).Type
		return Attribute{
			Description: bs.Description.Value,
			// TODO(vkleen): compute these values properly
			Optional:      true,
			Interpolation: InterpolationStrategy{InterpolationType: Nickel},
			Type: Type{
				Tag:     Dictionary,
				Content: &t,
			},
		}
	} else {
		obj := map[string]Attribute{}
		for k, v := range l.possible_values {
			obj[k] = assemble_blocks(key, bs, labels[1:], append(accumulated_bodies, &v))
		}
		return Attribute{
			Description: bs.Description.Value,
			// TODO(vkleen): compute these values properly
			Optional:      true,
			Interpolation: InterpolationStrategy{InterpolationType: Nickel},
			Type: Type{
				Tag:    Object,
				Object: &obj,
			},
		}
	}
}

func convert_block(key string, bs *schema.BlockSchema) Attribute {
	if bs.Body != nil && bs.Body.AnyAttribute != nil {
		if bs.Body.Blocks != nil || bs.Body.Attributes != nil {
			panic("Don't know how to handle AnyAttribute together with explicit attributes")
		}
		t := convert_attribute(bs.Body.AnyAttribute).Type
		return Attribute{
			Description: bs.Description.Value,
			// TODO(vkleen): compute these values properly
			Optional:      true,
			Interpolation: InterpolationStrategy{InterpolationType: Nickel},
			Type: Type{
				Tag:     Dictionary,
				Content: &t,
			},
		}
	}

	bodies := []*schema.BodySchema{}
	if bs.Body != nil {
		bodies = []*schema.BodySchema{bs.Body}
	}
	return assemble_blocks(key, bs, classify_labels(bs), bodies)
}

func assemble_bodies(bs ...*schema.BodySchema) map[string]Attribute {
	schemas := []map[string]Attribute{}
	for _, b := range bs {
		schemas = append(schemas, assemble_body(b))
	}
	return merge_objects(schemas...)
}

func assemble_body(bs *schema.BodySchema) map[string]Attribute {
	schema := make(map[string]Attribute)
	for key, attr := range bs.Attributes {
		schema[key] = convert_attribute(attr)
	}

	for key, block := range bs.Blocks {
		schema[key] = convert_block(key, block)
	}

	return schema
}

func main() {
  if len(os.Args) < 2 {
    panic("No provider schema directory passed")
  }

  schema_dir := os.Args[1]
  store, e := NewSchemaStore(os.DirFS(schema_dir))
  if e != nil {
    panic(e)
  }

	coreSchema, err := tfschema.CoreModuleSchemaForVersion(tfschema.LatestAvailableVersion)
	if err != nil {
		panic(err)
	}

	sm := tfschema.NewSchemaMerger(coreSchema)
	sm.SetSchemaReader(store)

	tf_schema, e := sm.SchemaForModule(&module.Meta{
		ProviderRequirements: store.ProviderReqs(),
    ProviderReferences: store.ProviderRefs(),
	})
	if e != nil {
		panic(e)
	}

  // log.Print(spew.Sdump(tf_schema))
	json, _ := json.Marshal(assemble_body(tf_schema))
	fmt.Println(string(json))
}
