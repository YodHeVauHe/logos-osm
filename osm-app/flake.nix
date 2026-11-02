{
  description = "osm-app — OSM distribution Basecamp app (LP-0018)";

  # Standalone flake so a module catalog (logos-modules-release-base fork)
  # can build this directory with `nix build .#lgx-portable`.
  # The repo-root flake builds the same module as `.#osm-app-*`.

  nixConfig = {
    extra-substituters = [ "https://cache.nix.logos.co/public" ];
    extra-trusted-public-keys = [ "public:l4HrXgL4nw246+LBh2SOJyhz64BoGegOYLheT/iIAPU=" ];
  };

  inputs = {
    # Same revision as the repo-root flake.lock.
    logos-module-builder.url = "github:logos-co/logos-module-builder/90444eb1f3388ec6244ea2f9f8295e98a710ef2d";
    # The osm core module from this repository (sibling directory).
    osm.url = "path:../osm-module";
    osm.inputs.logos-module-builder.follows = "logos-module-builder";
  };

  outputs = inputs@{ logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosQmlModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
