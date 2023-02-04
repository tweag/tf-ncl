let User = import "./user-contract.ncl" in
{
  TwoAdmins = fun label value =>
    let value = contract.apply {_: User} label value in
    let admins = value |> record.values |> array.filter (fun u => u.is-admin) in
    if array.length admins < 2
    then contract.blame_with "We need at least 2 admins" label
    else value,

  NoA = fun label value =>
      let value = contract.apply {_: Dyn} label value in
      let fields = record.fields value in
      if array.any (string.contains "a") fields
      then contract.blame_with "a is not allowed in usernames" label
      else value,
}
