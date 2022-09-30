{
  inputs = {
    nixpkgs.url = github:NixOS/nixpkgs;
    utils.url = github:numtide/flake-utils;
  };
  outputs = { self, utils, ... }@inputs: utils.lib.eachDefaultSystem (system: let
    pkgs = import "${inputs.nixpkgs}" {
      localSystem = { inherit system; };
      config = {};
    };
  in {
    devShell = pkgs.mkShell {
      buildInputs = with pkgs; [
        terraform
      ];
    };
  });
}
