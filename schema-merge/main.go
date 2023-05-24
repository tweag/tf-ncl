package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"

	//
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

func has_object_type(computed_fields *[]FieldDescriptor, path []string, ecs schema.ExprConstraints) (bool, *Type) {
	for _, ec := range ecs {
		switch c := ec.(type) {
		case schema.ObjectExpr:
			obj := map[string]Attribute{}
			for k, v := range c.Attributes {
				obj[k] = convert_attribute(computed_fields, append(path, k), v)
			}
			return true, &Type{
				Tag:    Object,
				Open:   false,
				Object: &obj,
			}
		}
	}
	return false, nil
}

func has_list_type(computed_fields *[]FieldDescriptor, path []string, ecs schema.ExprConstraints) (bool, *Type) {
	for _, ec := range ecs {
		switch c := ec.(type) {
		case schema.SetExpr:
			// TODO(vkleen) sets don't play well with the path concept
			content := extract_type(computed_fields, path, c.Elem)

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
			// TODO(vkleen) lists don't play well with the path concept
			content := extract_type(computed_fields, path, c.Elem)

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
func extract_type(computed_fields *[]FieldDescriptor, path []string, ecs schema.ExprConstraints) Type {
	if is_object, t := has_object_type(computed_fields, path, ecs); is_object {
		return *t
	}

	// FIXME(vkleen): the Nickel contracts don't react well to lists in the path; if we're going to emit an Array contract, we need to censor computed fields below it for now
	if is_list, t := has_list_type(&[]FieldDescriptor{}, path, ecs); is_list {
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

func extract_optional_computed(computed_fields *[]FieldDescriptor, path []string, as *schema.AttributeSchema) (bool, bool) {
	optional := as.IsOptional
	// Terraform treats `id` fields specially
	if path[len(path)-1] == "id" {
		optional = false
	}

	switch {
	case optional && !as.IsRequired && !as.IsComputed:
		return true, false
	case !optional && as.IsRequired && !as.IsComputed:
		// FIXME(vkleen): optional fields together with computed fields in the same record break the Nickel contracts
		return true, false
	case !optional && !as.IsRequired && as.IsComputed:
		*computed_fields = append(*computed_fields, FieldDescriptor{
			Force: true,
			Path:  append([]string(nil), path...),
		})
		// FIXME(vkleen): optional fields together with computed fields in the same record break the Nickel contracts
		return true, true
	case optional && !as.IsRequired && as.IsComputed:
		*computed_fields = append(*computed_fields, FieldDescriptor{
			Force: false,
			Path:  append([]string(nil), path...),
		})
		return true, true
	}
	return true, false
}

func convert_attribute(computed_fields *[]FieldDescriptor, path []string, as *schema.AttributeSchema) Attribute {
	t := extract_type(computed_fields, path, as.Expr)

	var o, c bool
	// FIXME(vkleen) Mark lists as "not computed" because of limitations in the Nickel contracts
	if t.Tag == List {
		o, c = true, false
	} else {
		o, c = extract_optional_computed(computed_fields, path, as)
	}

	attr := Attribute{
		Description: as.Description.Value,
		Optional:    o,
		Computed:    c,
		Type:        t,
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

func includes_wildcard_label(labels []Label) bool {
	for _, v := range labels {
		if v.wildcard {
			return true
		}
	}
	return false
}

func starts_with(pattern []string, path []string) bool {
	if len(path) < len(pattern) {
		return false
	}

	for i, v := range pattern {
		if v != path[i] {
			return false
		}
	}

	return true
}

func fixup_block_type(path []string, labels []Label, bs *schema.BlockSchema) schema.BlockType {
	t := bs.Type

	if len(path) == 1 && path[0] == "terraform" {
		t = schema.BlockTypeObject
	}

	if len(path) == 3 && starts_with([]string{"terraform", "backend"}, path) {
		t = schema.BlockTypeObject
	}

	has_wildcard := includes_wildcard_label(labels)
	if t == schema.BlockTypeNil && !has_wildcard {
		t = schema.BlockTypeList
	} else if t == schema.BlockTypeNil && has_wildcard {
		t = schema.BlockTypeObject
	}

	if bs.MaxItems == 1 {
		t = schema.BlockTypeObject
	}

	return t
}

func wrap_block_type(path []string, labels []Label, bs *schema.BlockSchema, t schema.BlockType, attr Attribute) Attribute {
	switch t {
	case schema.BlockTypeObject:
		return attr
	case schema.BlockTypeList, schema.BlockTypeSet:
		return Attribute{
			Description: attr.Description,
			Optional:    attr.Optional,
			Computed:    attr.Computed,
			Type: Type{
				Tag:      List,
				MinItems: &bs.MinItems,
				MaxItems: &bs.MaxItems,
				Content:  &attr.Type,
			},
		}
	default:
		panic(errors.New(fmt.Sprint("Unknown block type ", t.GoString())))
	}
}

var special_open_block_paths = [...][]string{{"data"}, {"resource"}, {"provider"}, {"terraform", "backend"}, {"module", "_"}}

func check_candidate(path []string, candidate []string) bool {
	if len(candidate) != len(path) {
		return false
	}
	for i := range candidate {
		if candidate[i] != path[i] {
			return false
		}
	}
	return true
}

func should_be_open(path []string) bool {
	for _, candidate := range special_open_block_paths {
		if check_candidate(path, candidate) {
			return true
		}
	}
	return false
}

func assemble_blocks(computed_fields *[]FieldDescriptor, path []string, bs *schema.BlockSchema, all_labels []Label, labels []Label, accumulated_bodies []*schema.BodySchema) Attribute {
	if len(labels) == 0 {
		t := fixup_block_type(path, all_labels, bs)

		// FIXME(vkleen): the Nickel contracts don't react well to lists in the path; if we're going to emit an Array contract, we need to censor computed fields below it for now
		var obj map[string]Attribute
		if t == schema.BlockTypeList || t == schema.BlockTypeSet {
			obj = assemble_bodies(&[]FieldDescriptor{}, path, accumulated_bodies...)
		} else {
			obj = assemble_bodies(computed_fields, path, accumulated_bodies...)
		}

		description := bs.Description.Value
		if len(accumulated_bodies) > 0 {
			last_description := accumulated_bodies[len(accumulated_bodies)-1].Description
			if last_description.Kind != lang.NilKind {
				description = last_description.Value
			}
		}
		return wrap_block_type(path, all_labels, bs, t, Attribute{
			Description: description,
			// TODO(vkleen): compute these values properly
			Optional: true,
			Computed: false,
			Type: Type{
				Tag:    Object,
				Open:   should_be_open(path),
				Object: &obj,
			},
		})
	}
	l := labels[0]
	if l.wildcard {
		t := assemble_blocks(computed_fields, append(path, "_"), bs, all_labels, labels[1:], accumulated_bodies).Type
		return Attribute{
			Description: bs.Description.Value,
			Optional:    true,
			Computed:    false,
			Type: Type{
				Tag:     Dictionary,
				Content: &t,
			},
		}
	} else {
		obj := map[string]Attribute{}
		for k, v := range l.possible_values {
			obj[k] = assemble_blocks(computed_fields, append(path, k), bs, all_labels, labels[1:], append(accumulated_bodies, &v))
		}
		return Attribute{
			Description: bs.Description.Value,
			// TODO(vkleen): compute these values properly
			Optional: true,
			Computed: false,
			Type: Type{
				Tag:    Object,
				Open:   should_be_open(path),
				Object: &obj,
			},
		}
	}
}

func convert_block(computed_fields *[]FieldDescriptor, path []string, bs *schema.BlockSchema) Attribute {
	if bs.Body != nil && bs.Body.AnyAttribute != nil {
		if bs.Body.Blocks != nil || bs.Body.Attributes != nil {
			panic("Don't know how to handle AnyAttribute together with explicit attributes")
		}
		t := convert_attribute(computed_fields, append(path, "_"), bs.Body.AnyAttribute).Type
		return Attribute{
			Description: bs.Description.Value,
			// TODO(vkleen): compute these values properly
			Optional: true,
			Computed: false,
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

	labels := classify_labels(bs)
	return assemble_blocks(computed_fields, path, bs, labels, labels, bodies)
}

func assemble_bodies(computed_fields *[]FieldDescriptor, path []string, bs ...*schema.BodySchema) map[string]Attribute {
	schemas := []map[string]Attribute{}
	for _, b := range bs {
		schemas = append(schemas, assemble_body(computed_fields, path, b))
	}
	return merge_objects(schemas...)
}

func assemble_body(computed_fields *[]FieldDescriptor, path []string, bs *schema.BodySchema) map[string]Attribute {
	schema := make(map[string]Attribute)
	for key, attr := range bs.Attributes {
		schema[key] = convert_attribute(computed_fields, append(path, key), attr)
	}

	for key, block := range bs.Blocks {
		schema[key] = convert_block(computed_fields, append(path, key), block)
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
		ProviderReferences:   store.ProviderRefs(),
	})
	if e != nil {
		panic(e)
	}

	computed_fields := []FieldDescriptor{}
	assembled_schema := assemble_body(&computed_fields, []string{}, tf_schema)
	json, err := json.Marshal(struct {
		ComputedFields []FieldDescriptor    `json:"computed_fields"`
		Schema         map[string]Attribute `json:"schema"`
	}{
		ComputedFields: computed_fields,
		Schema:         assembled_schema,
	})
	if err != nil {
		panic(err)
	}
	fmt.Println(string(json))
}
