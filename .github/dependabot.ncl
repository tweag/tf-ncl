{
  updates = [{
    commit-message = { include = "scope", prefix = "chore", },
    directory = "/",
    open-pull-requests-limit = 10,
    package-ecosystem = "github-actions",
    schedule = { interval = "daily", time = "00:00", timezone = "UTC", },
  }],
  version = 2,
}
