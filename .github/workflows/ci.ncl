{
  concurrency = { group = "ci-${{ github.ref }}", cancel-in-progress = true },
  name = "CI",
  on = { push = { branches = ["main"] }, pull_request = { branches = ["main"] } },
  jobs = {
    build-and-test = {
      strategy = { matrix = { os = [ "ubuntu-latest", "macos-latest" ] } },
      steps = [
        { uses = "actions/checkout@v3" },
        {
          uses = "cachix/install-nix-action@v20",
          name = "Installing Nix",
          with = {
            extra_nix_config = "experimental-features = nix-command flakes",
            nix_path = "nixpkgs=channel:nixos-unstable"
          }
        },
        { name = "Setup Cachix", with = { authToken = "${{ secrets.CACHIX_TWEAG_NICKEL_AUTH_TOKEN }}", name = "tweag-nickel" }, uses = "cachix/cachix-action@v12" },
        {
          run = "nix flake check --print-build-logs",
          name = "Run nix flake check"
        }
      ],
      runs-on = "${{ matrix.os }}"
    }
  }
}
