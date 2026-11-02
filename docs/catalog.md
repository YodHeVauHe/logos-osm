# Logos module catalog

The `osm` core module, the `osm-app` QML app and the SDK embedding examples
(`osm_consumer_example` and the `osm-consumer-app` QML app on top of it) are
published in a module catalog made from
[`logos-modules-release-base`](https://github.com/logos-co/logos-modules-release-base)
and released with
[`logos-modules-release-action`](https://github.com/logos-co/logos-modules-release-action).

**Catalog URL:**

```
https://raw.githubusercontent.com/YodHeVauHe/logos-osm-modules/main/logos-repo.json
```

- Catalog repository: <https://github.com/YodHeVauHe/logos-osm-modules>
- Index: <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/index/index.json>
- Current releases: [`osm-v1.0.3`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-v1.0.3),
  [`osm-app-v1.0.3`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-app-v1.0.3),
  [`osm_consumer_example-v1.0.2`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm_consumer_example-v1.0.2),
  [`osm-consumer-app-v1.0.0`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-consumer-app-v1.0.0)
- Earlier releases: [`osm-v1.0.2`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-v1.0.2),
  [`osm-app-v1.0.2`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-app-v1.0.2),
  [`osm-v1.0.1`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-v1.0.1),
  [`osm-app-v1.0.1`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-app-v1.0.1),
  [`osm-v1.0.0`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-v1.0.0),
  [`osm-app-v1.0.0`](https://github.com/YodHeVauHe/logos-osm-modules/releases/tag/osm-app-v1.0.0)

| Package | Version | Type | Variant | Size (bytes) | SHA-256 |
|---|---|---|---|---|---|
| `osm` | 1.0.3 | core | linux-amd64, darwin-arm64 | 12231485 | `bdf74fb43a35cc3d36fce237eb46b8b4bc6308cab4f3ecd7105daa457cce4bc0` |
| `osm-app` | 1.0.3 | ui_qml | linux-amd64, darwin-arm64 | 7849970 | `53e8a7b73ebc5a878563c1ba6aae6c13443b6accd64a72c61b62c16904931cb9` |
| `osm_consumer_example` | 1.0.2 | core | linux-amd64, darwin-arm64 | 7805648 | `7bded8d0df4a55d5e2250416f5980c3e3b37514e993b611771d1e7cfe78dbacc` |
| `osm-consumer-app` | 1.0.0 | ui_qml | linux-amd64, darwin-arm64 | 7805668 | `c6c910ac0d0b9c436ac87c8fa411f37c9cf6acd889ec7c5313d679754904f9fa` |
| `osm` | 1.0.2 | core | linux-amd64 | 6546944 | `9e410627533f931bf1daaf987f7df954a1c4cf3fad892f0f97df041cd8c9388c` |
| `osm-app` | 1.0.2 | ui_qml | linux-amd64 | 4210934 | `b530b767fca7116ab6123c49420e95fed9b2f7b280acefc449c2169b3252d145` |
| `osm` | 1.0.0 | core | linux-amd64 | 6555898 | `406704edbc167c8446814e5c963814a331c433486e3629621d50958353c99c16` |
| `osm-app` | 1.0.0 | ui_qml | linux-amd64 | 4197008 | `264d6fe918c2d08cd725532d2955342b98a85d575bbc3f5da02943af9b1f24f6` |

1.0.3 (and the two example packages) were built from the head of PR #18
(`build/darwin-ffi-and-example-locks`, commit `3bc74e9`): the catalog's
`submodules/logos-osm` is pinned to that commit (catalog commit `ad2b431`).
Each `.lgx` holds a `linux-amd64` and a `darwin-arm64` variant. The sizes and
SHA-256 values in the table are from the published `index.json`, and the
downloaded `.lgx` files hash to the same values
([`evidence/basecamp/catalog-sha256-check-1.0.3.txt`](evidence/basecamp/catalog-sha256-check-1.0.3.txt)).
The release runs:
[osm](https://github.com/YodHeVauHe/logos-osm-modules/actions/runs/41478659607),
[osm-app](https://github.com/YodHeVauHe/logos-osm-modules/actions/runs/41478714874),
[osm_consumer_example](https://github.com/YodHeVauHe/logos-osm-modules/actions/runs/41478732738),
[osm-consumer-app](https://github.com/YodHeVauHe/logos-osm-modules/actions/runs/41478752778).

1.0.2 was built from the head of PR #12 (`fix/osm-app-download-path-json`,
commit `6debd97`), not from `main`: the catalog's `submodules/logos-osm` is
pinned to that commit (catalog commit `6e8b675`). It carries the Download
path fix, escaped error replies and the consumer timeout. The sizes and
SHA-256 values above are from the published `index.json`; the downloaded
`.lgx` files hash to the same values.

Download URLs:

- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-v1.0.3/osm-1.0.3.lgx>
- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-app-v1.0.3/osm-app-1.0.3.lgx>
- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm_consumer_example-v1.0.2/osm_consumer_example-1.0.2.lgx>
- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-consumer-app-v1.0.0/osm-consumer-app-1.0.0.lgx>
- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-v1.0.2/osm-1.0.2.lgx>
- <https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-app-v1.0.2/osm-app-1.0.2.lgx>

Check a download against the index:

```sh
curl -sSLO https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-v1.0.3/osm-1.0.3.lgx
curl -sSLO https://github.com/YodHeVauHe/logos-osm-modules/releases/download/osm-app-v1.0.3/osm-app-1.0.3.lgx
sha256sum osm-1.0.3.lgx osm-app-1.0.3.lgx   # compare with the table above / index.json
```

Install with the Basecamp package manager by adding the catalog URL, or with
`lgpd`:

```sh
lgpd add-repo https://raw.githubusercontent.com/YodHeVauHe/logos-osm-modules/main/logos-repo.json
lgpd install osm-app   # pulls osm, which depends on storage_module
lgpd install osm-consumer-app   # pulls osm_consumer_example and osm
```

## Platforms: linux-amd64 and darwin-arm64

The `osm` module vendors the Rust FFI library in `osm-module/lib`:
`libosm_ffi.so` for x86_64 Linux and `libosm_ffi.dylib` for aarch64 macOS.
The `.dylib` is the `libosm_ffi-darwin-arm64` artifact of the `nix macOS` CI
job ([run 41469750144](https://github.com/YodHeVauHe/logos-osm/actions/runs/41469750144)).
CMake picks the `.so` on Linux and the `.dylib` on Apple, so each variant
carries only its own library. From 1.0.3 the catalog publishes `linux-amd64`
and `darwin-arm64`; the `darwin-arm64` builds run on `macos-latest` in the
release action. Releases up to 1.0.2 are `linux-amd64` only.

The `darwin-arm64` packages are built and published but were not loaded in
Basecamp on a Mac. Windows and Android are not published.

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
   - `release-osm-consumer-example.yml` with
     `module_path: submodules/logos-osm/examples/basecamp-consumer`
   - `release-osm-consumer-app.yml` with
     `module_path: submodules/logos-osm/examples/basecamp-consumer-app`
4. Set `variants` to `linux-amd64,darwin-arm64` in `_release-module.yml`
   (`linux-amd64` only up to 1.0.2; see above).
5. Set the catalog name and `indexUrl` in `logos-repo.json`, then ran each
   release workflow from the Actions tab. The action published the `.lgx`
   release and the `rebuild-index` workflow updated `index.json`.

Release each module through its own workflow. In one `release-all` run, the
artifact download pattern `lgx-osm-*` also matches `lgx-osm-app-*` and
`lgx-osm-consumer-app-*`, so the osm job fails with a manifest mismatch.

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
- The `osm` core module bundles `libosm_ffi.so` (or `.dylib`) next to the plugin; the FFI
  client finds it relative to the plugin `.so` at runtime.
- `osm-app` depends on `osm`, which depends on `storage_module`.
