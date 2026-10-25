{
  description = "LP-0018 OSM distribution for Logos — registry SPEL program, SDK, CLI, Basecamp module + app";

  inputs = {
    logos-module-builder.url = "github:logos-co/logos-module-builder";
    nixpkgs.follows = "logos-module-builder/nixpkgs";

    storage_module.url = "github:logos-co/logos-storage-module";
    storage_module.inputs.logos-module-builder.follows = "logos-module-builder";
  };

      outputs = inputs@{ self, nixpkgs, logos-module-builder, ... }:
    let
      # The FFI shared library is built outside nix via `make ffi`
      # (cargo build --release in ./ffi) and vendored into osm-module.
      # If the file doesn't exist, nix eval fails with a clear message.
      ffiLib = ./osm-module/vendored/libosm_ffi.so;
      osmModule = logos-module-builder.lib.mkLogosModule {
        src = ./osm-module;
        configFile = ./osm-module/metadata.json;
        flakeInputs = inputs;
        postInstall = ''
          mkdir -p $out/lib
          cp ${ffiLib} $out/lib/libosm_ffi.so
        '';
      };

      osmAppModule = logos-module-builder.lib.mkLogosQmlModule {
        src = ./osm-app;
        configFile = ./osm-app/metadata.json;
        flakeInputs = inputs // { osm = osmModule; };
      };

      packagesFor = system:
        let
          osmPkgs = osmModule.packages.${system} or {};
          appPkgs = osmAppModule.packages.${system} or {};
          pfx = tag: set: nixpkgs.lib.mapAttrs'
            (k: v: nixpkgs.lib.nameValuePair "${tag}-${k}" v) set;
        in
        pfx "osm" osmPkgs // pfx "osm-app" appPkgs;

      devShellsFor = system: {
        default = nixpkgs.legacyPackages.${system}.mkShell {
          buildInputs = with nixpkgs.legacyPackages.${system}; [
            cargo rustc rustfmt clippy rust-analyzer
            openssl
          ];
        };
      };
    in
    {
      packages = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ] packagesFor;
      devShells = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ] devShellsFor;
    };
}
