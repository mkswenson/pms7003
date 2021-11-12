{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    # Note: This has only been tested on x86_64-linux.
    utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages."${system}";
      naersk-lib = naersk.lib."${system}";
    in rec {
      # `nix build`
      packages.pm7003 = naersk-lib.buildPackage {
        pname = "pm7003";
        root = ./.;
        buildInputs = with pkgs; [ pkg-config udev ];
      };
      defaultPackage = packages.pm7003;

      overlay = final: prev: {
        pm7003 = packages.pm7003;
      };

      # `nix run`
      apps.my-project = utils.lib.mkApp {
        drv = packages.my-project;
      };
      defaultApp = apps.my-project;

      # `nix develop`
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [ rustc cargo ];
      };
    });
}
