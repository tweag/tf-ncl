let
  TaggedUnion = fun tag_field tags shapes label value =>
    let value = contract.apply { "%{tag_field}" | tags, .. } label value
    in contract.apply ({ "%{tag_field}" } & shapes value."%{tag_field}") label value
in {
  terraformTaggedUnion = `Foo,
  foo = 1,
  bar = "baz"
} | TaggedUnion "terraformTaggedUnion" [|`Foo, `Bar|] (match {
  `Foo => { foo | Num, bar | Str },
  `Bar => { foo | Str, baz | Str },
})
