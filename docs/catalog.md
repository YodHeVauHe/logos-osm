# Publishing to a Logos module catalog

The two Basecamp artifacts (`osm` core module, `osm-app` QML app) are
published as a fork of
[`logos-modules-release-base`](https://github.com/logos-co/logos-modules-release-base),
releasing via
[`logos-modules-release-action`](https://github.com/logos-co/logos-modules-release-action).
Evaluators install them through the Basecamp package-manager UI or `lgpd`
using the catalog's `logos-repo.json` URL.

## Steps

1. **Fork** `logos-modules-release-base` under the submitting organisation.
2. **Build both modules** (CI does this on every tag):
   ```sh
   cargo build --release -p osm_ffi
   mkdir -p osm-module/vendored
   cp target/release/libosm_ffi.so osm-module/vendored/
   nix build .#osm-install .#osm-app-install
   ```
3. **Copy the built module bundles** into the catalog fork's layout (the
   release action consumes `metadata.json` + built artifacts per module
   directory).
4. **Wire the release workflow** in the fork, modeled on the upstream
   `logos-modules-release-action` README:
   ```yaml
   name: release
   on:
     push:
       tags: ["osm-v*", "osm-app-v*"]
   jobs:
     release:
       uses: logos-co/logos-modules-release-action/.github/workflows/release.yml@main
       with:
         module: ${{ github.ref_name }}
   ```
5. **Commit and push the fork**; the action publishes a release and updates
   `logos-repo.json` in the fork root.
6. **Record the catalog URL** — the raw URL of the fork's `logos-repo.json`
   — in the README and the solution PR, so evaluators can:
   ```sh
   lgpd add-repo <logos-repo.json URL>
   lgpd install osm
   lgpd install osm-app
   ```

## notes

- Both modules are built with `logos-module-builder` (`mkLogosModule` /
  `mkLogosQmlModule`) — see `flake.nix`.
- The `osm` core module bundles `libosm_ffi.so` (Rust SDK FFI) next to the
  plugin; the FFI client locates it relative to the plugin .so at runtime.
- `osm-app` declares a dependency on `osm`, which declares a dependency on
  `storage_module` — installing `osm-app` pulls the full chain.
