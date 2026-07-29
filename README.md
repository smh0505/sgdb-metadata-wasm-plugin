# sgdb-metadata-wasm-plugin

A `MetadataProviderPlugin` for [Concourse](https://github.com/smh0505/Concourse) implemented as
a WASM component, ported from that project's built-in `sgdb.rs`. Fetches cover/background art
from [SteamGridDB](https://www.steamgriddb.com/) by title search - no description/genres/release
date, that's IGDB's job (see `igdb-metadata-wasm-plugin`).

This is a real, separate repo on purpose - same reasoning as `steam-source-wasm-plugin`: a
plugin whose source lives inside the host app's own repo doesn't genuinely exercise the
"install arbitrary third-party code" model the WASM plugin system is for.

This port needed a host primitive that didn't exist yet: `http-request` (method/url/headers/
body), since the existing `http-get` has no way to send SteamGridDB's required
`Authorization: Bearer <key>` header. Added to the shared `wit/plugin.wit` host interface and
implemented in the host app's `wasm_plugins.rs`, alongside a third `bindgen!`-generated world
(`metadata-plugin-world`) for this plugin kind.

Requires an API key - set it in Concourse's Settings under this plugin's row (rendered from
`plugin.json`'s `settingsSchema`, a generic form the host builds for any WASM plugin that
declares one; no custom UI code needed on either side). Stored via the same namespaced
`settings` table WASM plugins already use for their own scoped storage (`host::settings-get`/
`settings-set`), just written from the frontend directly using the identical key format instead
of round-tripping through a WASM call to set it.

## Permissions

Declares `httpScopes: ["steamgriddb.com"]` (Milestone 13 URL allowlisting) - covers
`www.steamgriddb.com` via subdomain match.

## Building

```sh
rustup target add wasm32-wasip1   # once
cargo install cargo-component     # once
cargo component build
```

Output: `target/wasm32-wasip1/debug/sgdb_metadata_wasm_plugin.wasm`.

## Installing into a running Concourse

Either build locally (above) or grab the prebuilt `.wasm` + `plugin.json` from this repo's
[Releases](https://github.com/smh0505/sgdb-metadata-wasm-plugin/releases) - CI (`.github/workflows/publish.yml`) publishes a new release
automatically whenever `plugin.json`'s `version` is bumped on `main`. Concourse's Settings ->
Metadata Provider tab -> Add Plugin also accepts a Release's `plugin.json` URL directly
(metadata-kind plugins install by URL, same as source plugins) - the latest one always lives
at:

```
https://github.com/smh0505/sgdb-metadata-wasm-plugin/releases/latest/download/plugin.json
```

Copy the compiled `.wasm` and `plugin.json` into
`<app data dir>/wasm-plugins/metadata/sgdb-wasm/` (Windows:
`%APPDATA%\com.bloppy.concourse\wasm-plugins\metadata\sgdb-wasm\`). It'll show up in Settings'
Plugins panel under the Metadata Provider tab next time the app starts, as
**SteamGridDB**.

## Versioning

Plain SemVer (`Cargo.toml` + `plugin.json`'s `version`), independent of Concourse's own
milestone-tracked version - patch for fixes, minor for backward-compatible new capabilities,
major for breaking manifest/WIT interface changes. Full convention:
[`.claude/CLAUDE.md`](https://github.com/smh0505/Concourse/blob/main/.claude/CLAUDE.md) (Plugin Versioning) in the main [Concourse](https://github.com/smh0505/Concourse) repo.
