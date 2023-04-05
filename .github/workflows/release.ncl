{
  name = "Release",
  on.release.types = ["created"],
  jobs = {
    "tarball" = {
      steps =
        import "../../ncl/github/setup-steps.ncl"
        @ [
          {
            name = "Create Release Tarball",
            run = "nix build .#release --print-build-logs",
          },
          {
            name = "Upload Release Tarball",
            uses = "actions/upload-release-asset@v1",
            with = {
              upload_url = "${{ github.event.release.upload_url }}",
              asset_path = "./result/tf-ncl.tar.xz",
              asset_name = "Generated Tf-Ncl Schemas",
              asset_content_type = "application/x-xz",
            },
          }
        ],
      runs-on = "ubuntu-latest",
    }
  },
}

