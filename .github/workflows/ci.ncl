{
  configs = [
    { os = "ubuntu-latest" },
    { os = "macos-latest" }
  ],

  concurrency = {
    group = "ci-${{ github.ref }}",
    cancel-in-progress = true
  },
  name = "CI",
  on = {
    push = {
      branches = ["main"]
    },
    pull_request = {
      branches = ["main"]
    }
  },

  matrix = {
    config | default = { os = "" },

    name = "Build and Test",

    job = {
      steps = import "../../ncl/github/setup-steps.ncl"
      @ [{
        run = "nix flake check --print-build-logs",
        name = "Run nix flake check"
      }],
      runs-on = config.os,
    }
  },
}
& import "../../ncl/github/matrix.ncl"
