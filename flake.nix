{
  description = "RGB lighting control for HyperX QuadCast S, QuadCast 2S, and DuoCast microphones";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      sliglight = pkgs.callPackage ./gui-package.nix { };
    in
    {
      packages.${system} = {
        default = sliglight;
        gui = sliglight;
      };

      nixosModules.default = import ./module.nix;

      overlays.default = _final: prev: {
        sliglight = prev.callPackage ./gui-package.nix { };
      };
    };
}
