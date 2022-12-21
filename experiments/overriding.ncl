let Overridable = fun ctr label value_ =>
  let with_label = fun c _l v => contract.apply c label v in
  if builtin.is_record value_ then
    let value_ = contract.apply { overridableRecordType | [| `Overridden, `Computed |], value }
      label value_
    in if value_.overridableRecordType == `Computed
    then contract.apply { overridableRecordType | [| `Computed |], value | with_label Str } label value_
    else contract.apply { overridableRecordType | [| `Overridden |], value | with_label ctr } label value_
  else
    { overridableRecordType = `Overridden, value | with_label ctr = value_}
in
let computed = fun value_ => { overridableRecordType = `Computed, value = value_ } in
let forced_field = fun type field_name l x => x |> record.map (fun name r => r & { "%{field_name}" | force = computed "${%{type}.%{name}.%{field_name}}"}) in

let rec remove_field_path | array.NonEmpty -> {_: Dyn} -> {_: Dyn}
                      = fun path r =>
    if array.length path == 1  && builtin.is_str (array.head path)
    then record.remove (array.head path) r
    else if array.length path == 1 && builtin.is_enum (array.head path) && array.head path == `Wildcard
    then {}
    else if builtin.is_enum (array.head path) && array.head path == `Wildcard
    then record.map (fun _name v => remove_field_path (array.tail path) v) r
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

let SchemaContract = {
  tls_private_key | {_: {
    id | Overridable Str,
    private_key_pem | Str | optional,
    public_key_pem | Overridable Str | optional | default = computed "public key for %{private_key_pem}",
  }} | forced_field "tls_private_key" "id",
  website | {_: {
    private_key | Str,
    certificate | Overridable Str,
    port | Overridable Num,
  }},
  server_port | Overridable Num | optional | default = computed "${server_port}",
  ..
} in

let Schema = fun label value =>
  contract.apply SchemaContract label value |> resolveOverridable |>
    remove_field_paths [["tls_private_key", `Wildcard, "id"]]
  in
{
  server_port = 9999,
  tls_private_key.test = {
    private_key_pem = website."nickel.org".private_key,
  },
  website."nickel.org" = {
    private_key = "my nickel.org private key for %{tls_private_key.test.id.value}",
    certificate = tls_private_key.test.public_key_pem,
    port = server_port,
  },
} | Schema
