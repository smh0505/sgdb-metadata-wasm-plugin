//! WASM `MetadataProviderPlugin` for SteamGridDB, ported from the game-library-client's
//! built-in `sgdb.rs`. Provides cover/background art only (no description/genres/release
//! date) - unlike IGDB, SteamGridDB's own API doesn't cover text metadata at all.
//!
//! Requires an API key, set via this plugin's `settingsSchema`-declared `api_key` setting
//! (see `plugin.json`) - read back here through `host::settings-get`, namespaced by the host
//! per plugin id so it can never collide with another plugin's settings.

#[allow(warnings)]
mod bindings;

use bindings::exports::gamelib::plugin::metadata_plugin::{Guest, MetadataResult};
use bindings::gamelib::plugin::host;

struct SgdbPlugin;

#[derive(serde::Deserialize)]
struct SearchResponse {
    data: Vec<SearchResult>,
}

#[derive(serde::Deserialize)]
struct SearchResult {
    id: u64,
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

fn search_game_id(auth: &str, title: &str) -> Result<Option<u64>, String> {
    let url = format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        urlencoding::encode(title)
    );
    let body = host::http_request("GET", &url, &[("Authorization".to_string(), auth.to_string())], None)?;
    let search: SearchResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(search.data.first().map(|g| g.id))
}

fn fetch_first_image(auth: &str, endpoint: &str, game_id: u64) -> Result<Option<String>, String> {
    let url = format!("https://www.steamgriddb.com/api/v2/{}/game/{}", endpoint, game_id);
    let body = host::http_request("GET", &url, &[("Authorization".to_string(), auth.to_string())], None)?;
    let images: ImageListResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(images.data.first().map(|i| i.url.clone()))
}

impl Guest for SgdbPlugin {
    fn fetch_metadata(title: String) -> Result<Option<MetadataResult>, String> {
        let auth = auth_header()?;

        let Some(game_id) = search_game_id(&auth, &title)? else {
            return Ok(None);
        };

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
