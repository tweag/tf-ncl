package main

import (
	"encoding/json"
)

type Attribute struct {
	Description   string                `json:"description"`
	Optional      bool                  `json:"optional"`
	Interpolation InterpolationStrategy `json:"interpolation"`
	Type          Type                  `json:"type"`
}

type InterpolationTypeTag uint64

const (
	Nickel = iota
	Terraform
)

type InterpolationStrategy struct {
	InterpolationType InterpolationTypeTag `json:"type"`
	Force             bool                 `json:"force"`
}

func (s InterpolationStrategy) MarshalJSON() (b []byte, e error) {
	switch s.InterpolationType {
	case Nickel:
		return json.Marshal(struct {
			Type string `json:"type"`
		}{
			Type: "nickel",
		})
	case Terraform:
		return json.Marshal(struct {
			Type  string `json:"type"`
			Force bool   `json:"force"`
		}{
			Type:  "terraform",
			Force: s.Force,
		})
	}
	return json.Marshal("unknown")
}

type TypeTag uint64

const (
	Dynamic = iota
	String
	Number
	Bool
	List
	Object
	Dictionary
)

func (t TypeTag) String() string {
	switch t {
	case Dynamic:
		return "Dynamic"
	case String:
		return "String"
	case Number:
		return "Number"
	case Bool:
		return "Bool"
	case List:
		return "List"
	case Object:
		return "Object"
	case Dictionary:
		return "Dictionary"
	}
	return "unknown"
}

type Type struct {
	Tag      TypeTag
	MinItems *uint64               `json:"min,omitempty"`
	MaxItems *uint64               `json:"max,omitempty"`
	Content  *Type                 `json:"content,omitempty"`
	Object   *map[string]Attribute `json:"object,omitempty"`
}

type ListVariant struct {
	MinItems *uint64 `json:"min,omitempty"`
	MaxItems *uint64 `json:"max,omitempty"`
	Content  *Type   `json:"content,omitempty"`
}

func (t Type) MarshalJSON() (b []byte, e error) {
	switch t.Tag {
	case List:
		return json.Marshal(struct {
			List ListVariant
		}{
			List: ListVariant{
				MinItems: t.MinItems,
				MaxItems: t.MaxItems,
				Content:  t.Content,
			},
		})
	case Object:
		return json.Marshal(struct {
			Object *map[string]Attribute
		}{
			Object: t.Object,
		})
	case Dictionary:
		return json.Marshal(struct {
			Dictionary *Type
		}{
			Dictionary: t.Content,
		})
	default:
		return json.Marshal(t.Tag.String())
	}
}

func merge_objects(os ...map[string]Attribute) map[string]Attribute {
	res := map[string]Attribute{}
	for _, o := range os {
		for k, v := range o {
			res[k] = v
		}
	}
	return res
}
