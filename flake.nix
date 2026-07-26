{
  description = "SUNix: GTK4 layer-shell software update popup";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, self }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.callPackage ./nix/shell.nix { };
        }
      );

      homeModules.default = import ./nix/hm.nix { inherit self; };

      overlays.default = final: prev: {
        sunix = prev.callPackage ./nix/drv.nix { };
      };

      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.callPackage ./nix/drv.nix { };
        }
      );
    };
}
