let name_of_config =
  fun config =>
    config
    |> record.values
    |> array.filter (builtin.is_string)
    |> string.join ", "
in
let sanitize = fun x =>
  function.pipe x [
    string.replace_regex "[^A-Za-z0-9-_]" "-",
    string.lowercase
  ]
in
{
  configs
    | Array { .. }
    | not_exported,
  matrix
    | not_exported
    | {
      name | String,
      job
        | {
          steps | Array { .. },
          ..
        },
      ..
    },
  jobs = configs
  |> array.map
    (
      fun config' =>
        {
          "%{sanitize "%{matrix.name}-%{name_of_config config'}"}" = (
            matrix & { config = config' }
          ).job & {
              name = "%{matrix.name}/(%{name_of_config config'})",
          }
        }
    )
  |> record.merge_all
}
