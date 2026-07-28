//! WASM `MetadataProviderPlugin` for SteamGridDB, ported from the game-library-client's
//! built-in `sgdb.rs`. Provides cover/background art only (no description/genres/release
//! date) - unlike IGDB, SteamGridDB's own API doesn't cover text metadata at all.
//!
//! Requires an API key, set via this plugin's `settingsSchema`-declared `api_key` setting
//! (see `plugin.json`) - read back here through `host::settings-get`, namespaced by the host
//! per plugin id so it can never collide with another plugin's settings.
//!
//! Two-step search-candidates/fetch-metadata-by-id split (metadata-plugin interface v2) rather
//! than one direct fetch - lets the host disambiguate when SteamGridDB's own autocomplete
//! search returns more than one plausible match instead of always committing to the first.
//!
//! Only listings whose `name` is an exact case-insensitive match to the query become
//! candidates at all, same reasoning as `rawg-metadata-wasm-plugin`/`igdb-metadata-wasm-plugin`
//! - a query with no exact-name match returns zero candidates (left blank by the host) rather
//! than a pile of only-loosely-related autocomplete suggestions.

#[allow(warnings)]
mod bindings;

use bindings::exports::gamelib::plugin::metadata_plugin::{Guest, MetadataCandidate, MetadataResult};
use bindings::gamelib::plugin::host;

struct SgdbPlugin;

#[derive(serde::Deserialize)]
struct SearchResponse {
    data: Vec<SearchResult>,
}

#[derive(serde::Deserialize)]
struct SearchResult {
    id: u64,
    name: String,
}

#[derive(serde::Deserialize)]
struct ImageListResponse {
    data: Vec<ImageEntry>,
}

#[derive(serde::Deserialize)]
struct ImageEntry {
    url: String,
}

fn auth_header() -> Result<String, String> {
    let key = host::settings_get("api_key")
        .ok_or_else(|| "SteamGridDB API key not set - configure it in Settings.".to_string())?;
    Ok(format!("Bearer {}", key))
}

fn fetch_first_image(auth: &str, endpoint: &str, game_id: u64) -> Result<Option<String>, String> {
    let url = format!("https://www.steamgriddb.com/api/v2/{}/game/{}", endpoint, game_id);
    let body = host::http_request("GET", &url, &[("Authorization".to_string(), auth.to_string())], None)?;
    let images: ImageListResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(images.data.first().map(|i| i.url.clone()))
}

impl Guest for SgdbPlugin {
    fn search_candidates(title: String) -> Result<Vec<MetadataCandidate>, String> {
        let auth = auth_header()?;
        let url = format!(
            "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
            urlencoding::encode(&title)
        );
        let body = host::http_request("GET", &url, &[("Authorization".to_string(), auth.clone())], None)?;
        let search: SearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;

        // SteamGridDB's own autocomplete response has no per-result image (confirmed via a real
        // API call - just id/name/verified/types/release_date), unlike IGDB/RAWG - so unlike
        // those two, a thumbnail here takes an extra grids lookup per candidate. Only done after
        // filtering to exact matches, so this is at most a handful of extra calls, not one per
        // raw autocomplete result. A candidate whose image lookup fails or comes up empty still
        // gets listed, just without a thumbnail - one bad lookup shouldn't drop a real candidate.
        Ok(search
            .data
            .into_iter()
            .filter(|g| g.name.eq_ignore_ascii_case(&title))
            .map(|g| {
                let image_url = fetch_first_image(&auth, "grids", g.id).ok().flatten();
                MetadataCandidate { id: g.id.to_string(), label: g.name, image_url }
            })
            .collect())
    }

    fn fetch_metadata_by_id(id: String) -> Result<Option<MetadataResult>, String> {
        let game_id: u64 = id.parse().map_err(|_| format!("Invalid SteamGridDB id: {}", id))?;
        let auth = auth_header()?;

        let cover_art_url = fetch_first_image(&auth, "grids", game_id)?;
        let background_art_url = fetch_first_image(&auth, "heroes", game_id)?;

        if cover_art_url.is_none() && background_art_url.is_none() {
            return Ok(None);
        }

        Ok(Some(MetadataResult {
            description: None,
            release_date: None,
            genres: vec![],
            cover_art_url,
            background_art_url,
        }))
    }
}

bindings::export!(SgdbPlugin with_types_in bindings);
