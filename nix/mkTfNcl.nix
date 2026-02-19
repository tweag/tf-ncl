{
  generateSchema,
  nixpkgs,
}:
{
  name ? "tf-ncl",
  nixpkgs,
  nickel ? nixpkgs.nickel,
  terraform ? nixpkgs.opentofu,
  terraformProviders ? nixpkgs.terraform-providers.actualProviders,
  providers ? (_: { }),
  extraNickelInput ? "",
  terraform-backend-git ? {
    repo = "";
    ref = "main";
  },
}:
let
  inherit (nixpkgs.stdenv) system;
  inherit (nixpkgs) lib writeShellApplication;
  prj-spec = builtins.fetchurl {
    url = "https://raw.githubusercontent.com/numtide/prj-spec/9b0ffcd0fddcb261bcd73ad9dad18a096760b4a0/contrib/direnv";
    sha256 = "1xwbvm1myy44zwv1l4f3acvns7zyq7blyib8y020vsbjd5l1m1p7";
  };

  terraform-with-plugins = terraform.withPlugins (p: nixpkgs.lib.attrValues (providers terraformProviders));
  
  devshell = nixpkgs.callPackage ./devshell.nix {
    inherit terraform generateSchema nickel terraformProviders;
  };
  ncl-schema = generateSchema terraformProviders terraform providers;
in
writeShellApplication {
  inherit name;
  runtimeEnv = {
    TF_IN_AUTOMATION = 1;
  };
  runtimeInputs = [
    nickel
    # nixpkgs.terraform-with-plugins
    nixpkgs.terraform-backend-git
  ]
  ++ (nixpkgs.lib.attrValues (devshell {
    inherit extraNickelInput providers;
  }))
  ++ lib.optional (terraform-backend-git.repo != "") terraform-backend-git;

  passthru = {
    inherit devshell;
  };
  text = ''
    set -e

    PATH_add() { export PATH="$1:$PATH"; }
    log_status() { echo "--- $*"; }

    # shellcheck disable=SC1091
    source ${prj-spec}
    export TF_PLUGIN_CACHE_DIR="$PRJ_CACHE_HOME/tf-plugin-cache"

    : "''${PRJ_DATA_DIR:=''${PRJ_DATA_HOME}}"

    if [[ ! -d "$PRJ_DATA_DIR"/tf-ncl/${name} ]]; then
       mkdir -p "$PRJ_DATA_DIR"/tf-ncl/${name}
       mkdir -p "$PRJ_CACHE_HOME"/tf-plugin-cache
    fi

    if [[ "$#" -le 1 ]]; then
      echo "terraform <ncl-file> ..."
      exit 1
    fi
    ENTRY="''${1}"
    shift
    ln -snfT ${ncl-schema} "$PRJ_DATA_DIR"/tf-ncl/${name}/schema.ncl
    nickel export > "$PRJ_DATA_DIR"/tf-ncl/${name}/main.tf.json <<EOF
      (import "''${ENTRY}").renderable_config
    EOF

    ${
      if terraform-backend-git.repo != "" then
        ''
          ENTRY_DIR="$(dirname "$ENTRY")"

          terraform-backend-git git \
             --dir "$PRJ_DATA_DIR"/tf-ncl/${name} \
             --repository ${terraform-backend-git.repo} \
             --ref ${terraform-backend-git.ref} \
             --state "''${ENTRY_DIR}/state.json" \
             ${lib.getExe terraform-with-plugins} "$@"
        ''
      else
        ''
          ${lib.getExe terraform-with-plugins} -chdir="$PRJ_DATA_DIR"/tf-ncl/${name} "$@"
        ''
    }
  '';
}
