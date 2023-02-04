{
  uniq | Array Dyn -> Array Dyn =
      let go : { visited : {_: {}}, out : Array Dyn } -> Dyn -> { visited : {_: {}}, out : Array Dyn } =
        fun acc nxt =>
          let as_str = builtin.serialize `Json nxt in
          if record.has_field as_str acc.visited then
            acc
          else
            { visited = record.insert as_str {} acc.visited, out = acc.out @ [nxt] }
      in fun arr =>
        (array.foldl go { visited = ({} | {_: {}}), out = [] } (arr | Array Dyn)).out,

  collect_teams = fun users => ["all"] @ (
    users
    |> record.values
    |> array.map (fun user => user.extra-teams)
    |> array.flatten
    |> uniq ),

  collect_users = fun users team =>
    users
    |> record.fields
    |> array.filter (fun user =>
      team == "all" || array.elem team users."%{user}".extra-teams),

  mk_teams = fun config users =>
    users
    |> collect_teams
    |> array.map (fun team => {
      resource.github_team."%{team}-team" = {
        name = team,
        privacy = "closed",
      },
      resource.github_team_members."%{team}-members" = {
        team_id = config.resource.github_team."%{team}-team".id,
        members = collect_users users team
          |> array.map (fun user => {
            username = user,
            role = if users."%{user}".is-admin
              then "maintainer"
              else "member",
          }),
      },
    })
    |> array.foldl (&) {},

  mk_memberships = fun users =>
    users
    |> record.fields
    |> array.map (fun name =>
      let user = users."%{name}" in
      {
        resource.github_membership."%{name}" = {
          role = if user.is-admin then "admin" else "member",
          username = name,
        }
      } 
    )
    |> array.foldl (&) {}
}
