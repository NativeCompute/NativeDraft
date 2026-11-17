{
  description = "NativeDraft, a free screenwriting app";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      each = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = each (pkgs: rec {
        nativedraft = pkgs.callPackage ./packaging/nix/package.nix { src = self; };
        default = nativedraft;
      });

      apps = each (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.nativedraft}/bin/nativedraft";
        };
      });
    };
}
