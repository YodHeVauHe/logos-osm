{
  description = "osm-consumer-app — Basecamp UI app built on the osm_consumer_example module (LP-0018)";

  # Standalone flake so the module catalog (logos-osm-modules) can build this
  # directory with `nix build .#lgx-portable`. The repo-root flake builds the
  # same app as `.#osm-consumer-app-*`.

  nixConfig = {
    extra-substituters = [ "https://cache.nix.logos.co/public" ];
    extra-trusted-public-keys = [ "public:l4HrXgL4nw246+LBh2SOJyhz64BoGegOYLheT/iIAPU=" ];
  };

  inputs = {
    # Same revision as the repo-root flake.lock.
    logos-module-builder.url = "github:logos-co/logos-module-builder/90444eb1f3388ec6244ea2f9f8295e98a710ef2d";
    osm_consumer_example.url = "path:../basecamp-consumer";
    osm_consumer_example.inputs.logos-module-builder.follows = "logos-module-builder";
  };

  outputs = inputs@{ logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosQmlModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
