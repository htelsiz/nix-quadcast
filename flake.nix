{
  description = "RGB lighting control for HyperX QuadCast S, QuadCast 2S, and DuoCast microphones";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      quadcastrgb = pkgs.callPackage ./package.nix { };
    in
    {
      packages.${system} = {
        default = quadcastrgb;
        cli = quadcastrgb;
        gui = pkgs.callPackage ./gui-package.nix { inherit quadcastrgb; };
      };

      nixosModules.default = import ./module.nix;

      overlays.default = _final: prev: {
        quadcastrgb = prev.callPackage ./package.nix { };
        quadcast-rgb-gui = prev.callPackage ./gui-package.nix {
          quadcastrgb = prev.callPackage ./package.nix { };
        };
      };
    };
}
