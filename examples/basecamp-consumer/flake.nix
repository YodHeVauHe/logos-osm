{
  description = "osm_consumer_example — independent consumer of the osm module (LP-0018)";

  # Standalone flake so the module catalog (logos-osm-modules) can build this
  # directory with `nix build .#lgx-portable`. The repo-root flake builds the
  # same module as `.#osm-consumer-example`.

  nixConfig = {
    extra-substituters = [ "https://cache.nix.logos.co/public" ];
    extra-trusted-public-keys = [ "public:l4HrXgL4nw246+LBh2SOJyhz64BoGegOYLheT/iIAPU=" ];
  };

  inputs = {
    # Same revision as the repo-root flake.lock.
    logos-module-builder.url = "github:logos-co/logos-module-builder/90444eb1f3388ec6244ea2f9f8295e98a710ef2d";
    osm.url = "path:../../osm-module";
    osm.inputs.logos-module-builder.follows = "logos-module-builder";
  };

  outputs = inputs@{ logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
