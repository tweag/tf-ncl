package main

import (
  "encoding/json"
)

type Attribute struct {
  Description string `json:"description"`
  Optional bool `json:"optional"`
  Interpolation InterpolationStrategy `json:"interpolation"`
  Type Type `json:"type"`
}

type InterpolationStrategy struct {
  InterpolationType string `json:"type"`
  Force *bool `json:"force,omitempty"`
}

type Type interface {
  MinItems() *uint32
  MaxItems() *uint32
  Content() *Type
  Object() map[string]Attribute
}

type Dynamic struct {}

func (t Dynamic) MinItems() *uint32 {
  return nil
}

func (t Dynamic) MaxItems() *uint32 {
  return nil
}

func (t Dynamic) Content() *Type {
  return nil
}

func (t Dynamic) Object() map[string]Attribute {
  return make(map[string]Attribute, 0)
}

func (t Dynamic) MarshalJSON() (b []byte, e error) {
  return json.Marshal(map[string]string{
    "type": "Dynamic",
  })
}

type String struct {}

func (t String) MinItems() *uint32 {
  return nil
}

func (t String) MaxItems() *uint32 {
  return nil
}

func (t String) Content() *Type {
  return nil
}

func (t String) Object() map[string]Attribute {
  return make(map[string]Attribute, 0)
}

func (t String) MarshalJSON() (b []byte, e error) {
  return json.Marshal(map[string]string{
    "type": "String",
  })
}

type Number struct {}

func (t Number) MinItems() *uint32 {
  return nil
}

func (t Number) MaxItems() *uint32 {
  return nil
}

func (t Number) Content() *Type {
  return nil
}

func (t Number) Object() map[string]Attribute {
  return make(map[string]Attribute, 0)
}

func (t Number) MarshalJSON() (b []byte, e error) {
  return json.Marshal(map[string]string{
    "type": "Number",
  })
}

type Bool struct {}

func (t Bool) MinItems() *uint32 {
  return nil
}

func (t Bool) MaxItems() *uint32 {
  return nil
}

func (t Bool) Content() *Type {
  return nil
}

func (t Bool) Object() map[string]Attribute {
  return make(map[string]Attribute, 0)
}

func (t Bool) MarshalJSON() (b []byte, e error) {
  return json.Marshal(map[string]string{
    "type": "Bool",
  })
}

type List struct {
  minItems *uint32
  maxItems *uint32
  content *Type
}

type Object struct {
  object map[string]Attribute
}

type Dictionary struct {
  content *Type
}
