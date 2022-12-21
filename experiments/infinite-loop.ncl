let rec remove_field_path | array.NonEmpty -> {_: Dyn} -> {_: Dyn}
                      = fun path r =>
    if array.length path == 1  && builtin.is_str (array.head path)
    then record.remove (array.head path) r
    else if array.length path == 1 && builtin.is_enum (array.head path) && array.head path == `Wildcard
    then {}
    else if builtin.is_enum (array.head path) && array.head path == `Wildcard
    then record.map (fun _name v => remove_field_path (array.tail path) v) r
    else if !record.has_field (array.head path) r
    then r
    else r & { "%{array.head path}" | force = remove_field_path (array.tail path) r."%{array.head path}" }
  in
let remove_field_paths = fun paths r =>
    array.foldl (fun rec_acc path => remove_field_path path rec_acc) r paths
  in
let isOverridableRecord = fun r => builtin.is_record r && record.has_field "overridableRecordType" r in
let rec resolveOverridable = fun r => r |>
  record.map (fun _name field =>
    if builtin.is_record field
    then if isOverridableRecord field
      then field.value
      else resolveOverridable field
    else field)
  in



let fail = fun msg =>
  let FailWithTrace = fun label value => contract.blame_with msg label in (null | FailWithTrace)
  in

let
  TerraformReference = Array Str
  in

let
  is_terraform_field_record
    : Dyn -> Bool
    = fun v =>
      builtin.is_record v && record.has_field "terraform_field_type" (v | {_: Dyn})
  in

let TerraformField = fun ctr label value_ =>
  let with_label = fun c _l v => contract.apply c label v in
  if is_terraform_field_record value_ then
    let value_ = contract.apply { terraform_field_type | [| `Literal, `Reference, `ProviderComputed |], .. } label value_
    in contract.apply (value_.terraform_field_type |> match {
      `Literal => { terraform_field_type | [| `Literal |], value | with_label ctr },
      `Reference => { terraform_field_type | [| `Reference |], value | with_label (Array Dyn) },
      `ProviderComputed => { terraform_field_type | [| `ProviderComputed |], path | with_label TerraformReference}
    }) label value_
  else
    { terraform_field_type = `Literal, value | with_label ctr = value_ }
  in

let provider_computed = fun path_ =>
    { terraform_field_type = `ProviderComputed, path = path_ }
  in

let
  resolve_terraform_field_at
    | Array Str -> Dyn -> Dyn
    = fun path field => field.terraform_field_type |> match {
        `Literal => field.value,
        `Reference => field,
        `ProviderComputed => if path == field.path 
          then { terraform_field_to_be_deleted = true }
          else {
            terraform_field_type = `Reference,
            value = field.path,
          }
      }
  in

let
  resolve_provider_computed
    = let rec go = fun acc_path r => 
        r |> record.map (fun name field =>
            if is_terraform_field_record field
              then resolve_terraform_field_at (acc_path @ [name]) field | Dyn
            else if builtin.is_record field
              then go (acc_path @ [name]) field | Dyn
            else acc_path @ [name] | Dyn
          )
      in fun r => go [] r
  in

let
  removable_fields = fun r =>
    array.foldl (fun acc n => 
      if builtin.is_record r."%{n}" && record.has_field "terraform_field_to_be_deleted" r."%{n}"
      then acc @ [n]
      else acc
    ) [] (record.fields r)
  in

let
  remove_fields = fun fields r =>
    array.foldl (fun r n => record.remove n r) r fields
  in

let rec
  delete_fields = fun r =>
    r |> remove_fields (removable_fields r) |> record.map (fun name field =>
      if builtin.is_record field
      then delete_fields field
      else field
    )
  in

let
  resolve_reference
    | Array Str -> Str
    = fun ns => "${%{string.join "."}}"
  in

let rec
  resolve_references = fun r => 
    if is_terraform_field_record r
    then if r.terraform_field_type != `Reference then fail "Expected only Terraform references"
         else resolve_reference r
    else if !builtin.is_record r
    then r
    else resolve_references
  in

let SchemaContract = {
  test | { res | {
    connection | {
      target | TerraformField Str,
    },
    ip | TerraformField Str
      | default = provider_computed ["test", "res", "ip"],
  }},
  ..
} in

let Schema = fun label value =>
  contract.apply SchemaContract label value
    |> resolve_provider_computed |> delete_fields |> resolve_references
  in
[{
  test.res = {
    connection.target = test.res.ip,
  },
} | Schema,
{
  test.res = {
    connection.target = test.res.ip,
    ip = "127.0.0.1"
  },
} | Schema,
]
