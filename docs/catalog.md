# Logos module catalog

The two Basecamp artifacts, the `osm` core module and the `osm-app` QML app,
are published in a module catalog made from
[`logos-modules-release-base`](https://github.com/logos-co/logos-modules-release-base)
and released with
[`logos-modules-release-action`](https://github.com/logos-co/logos-modules-release-action).

**Catalog URL:**

```
https://raw.githubusercontent.com/YodHeVauHe/logos-osm-modules/main/logos-repo.json
```

- Catalog repository: <https://github.com/YodHeVauHe/logos-osm-modules>
- Index: <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/index/index.json>
- Current releases: [`osm-v1.0.1`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-v1.0.1),
  [`osm-app-v1.0.1`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-app-v1.0.1)
- Earlier releases: [`osm-v1.0.0`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-v1.0.0),
  [`osm-app-v1.0.0`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-app-v1.0.0)

| Package | Version | Type | Variant | Size (bytes) | SHA-256 |
|---|---|---|---|---|---|
| `osm` | 1.0.1 | core | linux-amd64 | 6543855 | `3ededd2d2469591b25cfa6b1a342758c44c46d48ebc3d44b36bf84a227fc8b6f` |
| `osm-app` | 1.0.1 | ui_qml | linux-amd64 | 4210184 | `df760d733f6c0cb2a7904a0a19b9e407ef847b9964637f505473340e6c70b430` |
| `osm` | 1.0.0 | core | linux-amd64 | 6555898 | `406704edbc167c8446814e5c963814a331c433486e3629621d50958353c99c16` |
| `osm-app` | 1.0.0 | ui_qml | linux-amd64 | 4197008 | `264d6fe918c2d08cd725532d2955342b98a85d575bbc3f5da02943af9b1f24f6` |

1.0.1 carries the Basecamp fixes from #10 (module backend for `osm-app`,
rebuilt `libosm_ffi.so`). Download URLs:

- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-v1.0.1/osm-1.0.1.lgx>
- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-app-v1.0.1/osm-app-1.0.1.lgx>

Check a download against the index:

```sh
curl -sSLO https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-v1.0.1/osm-1.0.1.lgx
curl -sSLO https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-app-v1.0.1/osm-app-1.0.1.lgx
sha256sum osm-1.0.1.lgx osm-app-1.0.1.lgx   # compare with the table above / index.json
```

Install with the Basecamp package manager by adding the catalog URL, or with
`lgpd`:

```sh
lgpd add-repo https://raw.githubusercontent.com/YodHeVauHe/logos-osm-modules/main/logos-repo.json
lgpd install osm-app   # pulls osm, which depends on storage_module
```

## Limitation: linux-amd64 only

The `osm` module ships the vendored `osm-module/lib/libosm_ffi.so`, which is
built for x86_64 Linux. The catalog therefore publishes only the
`linux-amd64` variant. macOS and aarch64 builds need a per-platform FFI
library and are not published.

## How the catalog was built

1. Created `YodHeVauHe/logos-osm-modules` from the
   `logos-modules-release-base` template ("Use this template").
2. Added this repository as the submodule `submodules/logos-osm`, pinned to a
   commit that contains standalone flakes in `osm-module/flake.nix` and
   `osm-app/flake.nix`. The release action runs `nix build .#lgx-portable`
   inside each module directory, so each directory needs its own flake. The
   root `flake.nix` keeps working as before.
3. Added one release workflow per module:
   - `release-osm.yml` with `module_path: submodules/logos-osm/osm-module`
   - `release-osm-app.yml` with `module_path: submodules/logos-osm/osm-app`
4. Limited `variants` to `linux-amd64` in `_release-module.yml` (see above).
5. Set the catalog name and `indexUrl` in `logos-repo.json`, then ran each
   release workflow from the Actions tab. The action published the `.lgx`
   release and the `rebuild-index` workflow updated `index.json`.

Release osm and osm-app through their separate workflows. In one
`release-all` run, the artifact download pattern `lgx-osm-*` also matches
`lgx-osm-app-*`, so the osm job fails with a manifest mismatch.

## Reproduce the bundles locally

```sh
(cd osm-module && nix build .#lgx-portable)
(cd osm-app && nix build .#lgx-portable)
# or from the repository root:
nix build .#osm-lgx-portable .#osm-app-lgx-portable
```

## Notes

- Both modules are built with `logos-module-builder` (`mkLogosModule` /
  `mkLogosQmlModule`).
- The `osm` core module bundles `libosm_ffi.so` next to the plugin; the FFI
  client finds it relative to the plugin `.so` at runtime.
- `osm-app` depends on `osm`, which depends on `storage_module`.
