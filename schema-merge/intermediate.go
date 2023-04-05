package main

import (
	"encoding/json"
)

type Attribute struct {
	Description string `json:"description"`
	Optional    bool   `json:"optional"`
	Computed    bool   `json:"computed"`
	Type        Type   `json:"type"`
}

type FieldDescriptor struct {
	Force bool     `json:"force"`
	Path  []string `json:"path"`
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
	Open     bool                  `json:"open,omitempty"`
	Object   *map[string]Attribute `json:"object,omitempty"`
}

type ListVariant struct {
	MinItems *uint64 `json:"min,omitempty"`
	MaxItems *uint64 `json:"max,omitempty"`
	Content  *Type   `json:"content,omitempty"`
}

type ObjectVariant struct {
	Open    bool                  `json:"open"`
	Content *map[string]Attribute `json:"content,omitempty"`
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
			Object ObjectVariant
		}{
			Object: ObjectVariant{
				Open:    t.Open,
				Content: t.Object,
			},
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
