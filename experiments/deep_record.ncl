(let deep_map_filter = {
  go : Array Str -> (Array Str -> Dyn -> {action: [| `KeepStop, `Continue, `Delete |], value : Dyn}) -> {_ : Dyn} -> {_ : Dyn} -> Str -> {_: Dyn} = fun prefix f r acc field_name => 
    let prefix = prefix @ [field_name] in 
    let result = f prefix (r."%{field_name}") in
    result.action |> match {
      `KeepStop => record.insert field_name result.value acc,
      `Continue => 
        let new_value : Dyn = 
          if builtin.is_record result.value then
            (deep_map_filter_prefix prefix f (result.value | {_ : Dyn})) | Dyn
          else
            result.value in
        record.insert field_name new_value acc,
      `Delete => acc,
    },

  deep_map_filter_prefix : Array Str -> (Array Str -> Dyn -> {action: [| `KeepStop, `Continue, `Delete |], value : Dyn}) -> {_ : Dyn} -> {_ : Dyn}
      = fun prefix f r =>
        (r | {; Dyn})
        |> record.fields
        |> array.foldl (go prefix f r) {},

  deep_map_filter : (Array Str -> Dyn -> {action: [| `KeepStop, `Continue, `Delete |], value : Dyn}) -> {_ : Dyn} -> {_ : Dyn}
      = deep_map_filter_prefix [],
}.deep_map_filter in  

({foo.bar = 1, foo.baz.bar.blorg = 2, bar = 3} | {_: Dyn})
|> deep_map_filter (fun path value_ =>
    if array.length path >= 3
        then { action = `Delete, value = value_ }
    else if path == ["bar", "foo"]
        then { action = `KeepStop, value = value_ }
    else if path == ["bar"]
        then { action = `Continue, value = { foo = { eh = value_ } } | Dyn }
    else { action = `Continue, value = value_ })
) : _
