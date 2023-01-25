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
    TaggedUnion "terraform_field_type" [| `Undefined, `Literal, `Reference, `ProviderComputed |] (match {
      `Undefined => {},
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

  undefined = { terraform_field_type = `Undefined },

  resolve_reference
    : Array Str -> Str
    = fun ns => "${%{string.join "." ns}}",

  typeof
    : Dyn -> [| `TerraformField, `Array, `Other |]
    = fun v =>
      if is_terraform_field_record v
      then `TerraformField
      else if builtin.is_array v
      then `Array
      else `Other,

  resolve_provider_computed =
    let {go, resolve_field_action, ..} = {
      resolve_field_action | Array Str -> TerraformField Dyn -> Dyn
        = fun path field => field.terraform_field_type |> match {
            `Undefined => {
              action = `Delete, value = null
            },
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
          },
      go | Array Str -> Dyn -> { action: [| `KeepStop, `Continue, `Delete |], value: Dyn }
        = fun path field => typeof field |> match {
            `TerraformField => resolve_field_action path field,
            `Array => {
              action = `KeepStop, value = array.map resolve_provider_computed field
            },
            `Other => {
              action = `Continue, value = field
            }
          },
    } in fun v => 
      if is_terraform_field_record v
      then (resolve_field_action [] v).value
      else deep_filter_map go v,

  is_defined_terraform_field_record | Dyn -> Bool
    = fun x => is_terraform_field_record x && x.terraform_field_type != `Undefined,

  has_defined_field | Str -> {_: Dyn} -> Bool
    = fun name r => record.has_field name r && is_defined_terraform_field_record r."%{name}",

  has_defined_field_path | Array Str -> { _: Dyn } -> Bool
    = fun path r =>
      if array.length path == 1
      then has_defined_field (array.head path) r
      else
        let field = array.head path
        in    record.has_field field r
           && builtin.is_record r."%{field}"
           && has_defined_field_path (array.tail path) r."%{field}",

  FieldDescriptor = {
    prio | [| `Default, `Force |],
    path | Array Str,
  },

  ElaboratedField = {
    prio | [| `Default, `Force |],
    path | Array Str,
    record | {_: Dyn},
  },

  elaborate_field | {_: Dyn} -> FieldDescriptor -> Array ElaboratedField
    = let { candidates, filter_existing, elaborate, .. } = {
      candidates | {_: Dyn}  -> Array Str -> FieldDescriptor -> Array FieldDescriptor
        = fun r prefix field => 
          let head = array.head field.path in
          let tail = array.tail field.path in
            if head == "_"
            then record.fields r |> array.map (fun name => candidates r prefix { prio = field.prio, path = [name] @ tail }) |> array.flatten
            else if array.length field.path == 1
              then
                [{
                  prio = field.prio,
                  path = prefix @ [head]
                }]
              else
                let r' = if record.has_field head r then r."%{head}" else {}
                in candidates r' (prefix @ [head]) { prio = field.prio, path = tail },
      
      filter_existing | {_: Dyn} -> Array FieldDescriptor -> Array FieldDescriptor
        = fun r => array.filter (fun field => ! (has_defined_field_path field.path r && field.prio == `Default)),

      elaborate | FieldDescriptor -> ElaboratedField
        = fun field => {
          prio = field.prio,
          path = field.path,
          record = elaborated_record field.path field
        },

      elaborated_record | Array Str -> FieldDescriptor -> {_: Dyn}
        = fun acc field =>
          let head = array.head acc
          in if array.length acc > 1
          then { "%{head}" = elaborated_record (array.tail acc) field }
          else { "%{head}" | force = provider_computed field.path },
    } in fun r field => candidates r [] field |> filter_existing r |> array.map elaborate,

  merge_elaborated | {_: Dyn} -> ElaboratedField -> {_: Dyn}
    = fun record field =>
      if field.prio == `Force || ! has_defined_field_path field.path record
      then record & field.record
      else record,

  last : forall a. Array a -> a
    = fun arr => array.elem_at (array.length arr - 1) arr,

  foldl' : forall a b. (a -> b -> a) -> a -> Array b -> a
    = fun f acc l =>
        let length = array.length l in
        if length == 0 then acc
        else
          let rec
            out = [ acc ] @ array.generate
              (fun i => f (array.elem_at i out) (array.elem_at i l))
              length
          in array.elem_at length out,

  flatten' : forall a. Array (Array a) -> Array a
    = fun l => foldl' (fun acc l => acc @ l) [] l,

  ComputedFields | Array FieldDescriptor -> Dyn -> Dyn -> Dyn
    = fun fields _l r => fields |> array.map (elaborate_field r) |> flatten' |> foldl' merge_elaborated r,
}