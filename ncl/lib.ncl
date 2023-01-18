{
  deep_filter_map = {
    go : Array Str -> (Array Str -> Dyn -> {action: [| `KeepStop, `Continue, `Delete |], value : Dyn}) -> {_ : Dyn} -> {_ : Dyn} -> Str -> {_: Dyn} = fun prefix f r acc field_name => 
      let prefix = prefix @ [field_name] in 
      let result = f prefix (r."%{field_name}") in
      result.action |> match {
        `KeepStop => record.insert field_name result.value acc,
        `Continue => 
          let new_value : Dyn = 
            if builtin.is_record result.value then
              (deep_filter_map_prefix prefix f (result.value | {_ : Dyn})) | Dyn
            else
              result.value in
          record.insert field_name new_value acc,
        `Delete => acc,
      },
  
    deep_filter_map_prefix : Array Str -> (Array Str -> Dyn -> {action: [| `KeepStop, `Continue, `Delete |], value : Dyn}) -> {_ : Dyn} -> {_ : Dyn}
        = fun prefix f r =>
          r
          |> record.fields
          |> array.foldl (go prefix f r) {},
  
    deep_filter_map : (Array Str -> Dyn -> {action: [| `KeepStop, `Continue, `Delete |], value : Dyn}) -> {_ : Dyn} -> {_ : Dyn}
        = deep_filter_map_prefix [],
  }.deep_filter_map,


  TaggedUnion = fun tag_field tags shapes label value =>
    let value = contract.apply { "%{tag_field}" | tags, .. } label value
    in contract.apply ({ "%{tag_field}" } & shapes value."%{tag_field}") label value,


  TerraformReference = Array Str,

  TerraformField = fun ctr => 
    TaggedUnion "terraform_field_type" [| `Literal, `Reference, `ProviderComputed |] (match {
      `Literal => { value | ctr },
      `Reference => { value | Array Dyn },
      `ProviderComputed => { path | TerraformReference },
    }),

  is_terraform_field_record
    : Dyn -> Bool
    = fun v =>
      builtin.is_record v && record.has_field "terraform_field_type" (v | {_: Dyn}),

  Tf = fun ctr label value_ =>
    if is_terraform_field_record value_
    then contract.apply (TerraformField ctr) label value_
    else { terraform_field_type = `Literal, value | (fun _l v => contract.apply ctr label v) = value_},

  provider_computed = fun path_ =>
    { terraform_field_type = `ProviderComputed, path = path_ },

  resolve_reference
    : Array Str -> Str
    = fun ns => "${%{string.join "." ns}}",

  resolve_provider_computed = 
    let
      go | Array Str -> Dyn -> { action: [| `KeepStop, `Continue, `Delete |], value: Dyn }
        = fun path field =>
          if is_terraform_field_record field
          then
            field.terraform_field_type |> match {
              `Literal => {
                action = `KeepStop,
                value = field.value
              },
              `Reference => {
                action = `KeepStop,
                value = resolve_reference field.value
              },
              `ProviderComputed =>
                if path == field.path 
                then { action = `Delete, value = null }
                else {
                  action = `KeepStop,
                  value = resolve_reference field.path
                }
            }
          else { action = `Continue, value = field }
    in deep_filter_map go,

  computed_field_default = fun prefix path =>
    let p = array.head path in
      if array.length path == 1
      then { "%{p}" | default = provider_computed (prefix @ [p]) }
      else { "%{p}" | default = computed_field_default (prefix @ [p]) (array.tail path)},

  has_field_path | Array Str -> { _: Dyn } -> Bool
    = fun path r =>
      if array.length path == 1
      then record.has_field (array.head path) r
      else
        let field = array.head path
        in    record.has_field field r
           && builtin.is_record r."%{field}"
           && has_field_path (array.tail path) r."%{field}",

  add_computed_field_defaults = fun prefix paths l r =>
    r |> record.map (fun name r =>
        array.foldl (fun r path => 
          if has_field_path path r
          then r
          else r & computed_field_default (prefix @ [name]) path
        ) r paths
      ),
}
