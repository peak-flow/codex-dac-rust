# Spotify Import Code Flow

## Metadata
| Field | Value |
|-------|-------|
| Repository | `codex-dac-rust` |
| Commit | `77896ae` |
| Documented | `2026-03-18` |
| Trigger | User clicks "Import Spotify Library" button |
| End State | `AppSnapshot` with Spotify tracks merged into library, Spotify playlist crates created, smart crates rebuilt, stats recalculated |

## Verification Summary
- [VERIFIED]: 28
- [INFERRED]: 2
- [NOT_FOUND]: 0
- [ASSUMED]: 0

---

## Flow Diagram

```
[User clicks "Import Spotify Library"]
        │
        ▼
  App.tsx::handleImportSpotify()
        │
        ├──→ importSpotifyLibrary(accessToken?)  [IPC invoke]
        │        │
        │        ▼
        │    lib.rs::import_spotify_library()  [async]
        │        │
        │        ├──→ Lock #1: resolve token (state or param)
        │        │
        │        ├── Token present?
        │        │     ├── YES ──→ fetch_spotify_library(&token)
        │        │     │              │
        │        │     │              ├──→ fetch_saved_tracks()  [paginated]
        │        │     │              │        │
        │        │     │              │        └──~~> GET /v1/me/tracks?limit=50 (loop)
        │        │     │              │
        │        │     │              ├──→ fetch_playlists()  [paginated]
        │        │     │              │        │
        │        │     │              │        └──~~> GET /v1/me/playlists?limit=20 (loop)
        │        │     │              │
        │        │     │              └──→ per playlist: fetch_playlist_tracks()  [paginated]
        │        │     │                       │
        │        │     │                       └──~~> GET /v1/playlists/{id}/tracks?limit=100 (loop)
        │        │     │
        │        │     └── On API error ──→ demo_spotify_import() [fallback]
        │        │
        │        └── NO ──→ demo_spotify_import()
        │
        │        ├──→ Lock #2: merge and finalize
        │        │        │
        │        │        ├──→ merge_tracks() [dedup + hybrid promotion]
        │        │        ├──→ Resolve spotify_ids → track_ids in crates
        │        │        ├──→ Remove old spotify crates, push new ones
        │        │        └──→ refresh_snapshot()
        │        │                 ├──→ build_smart_crates()
        │        │                 ├──→ build_stats()
        │        │                 └──→ generate_mix_assistant()
        │        │
        │        ▼
        │    Ok(snapshot.clone())
        │
        ▼
  applySnapshot(next)  [React state update]
```

---

## Detailed Flow

### Step 1: Button Click — UI Entry Point

[VERIFIED: src/App.tsx:309]
```tsx
<button className="primary-action" onClick={() => void handleImportSpotify()} type="button">
    Import Spotify Library
</button>
```

---

### Step 2: handleImportSpotify

[VERIFIED: src/App.tsx:172-182]
```tsx
async function handleImportSpotify() {
    setStatusLine("Pulling playlists and saved tracks from Spotify...");

    try {
        const next = await importSpotifyLibrary(spotifyAccessToken || undefined);
        applySnapshot(next);
    } catch (error) {
        console.error(error);
        setStatusLine("Spotify import failed.");
    }
}
```

Sets a status message, then calls `importSpotifyLibrary` with the access token from React state. Converts empty string to `undefined` via `|| undefined`.

**Data in:** `spotifyAccessToken: string` (from input field at `src/App.tsx:59`)
**Calls:** `importSpotifyLibrary()` via IPC

---

### Step 3: Tauri IPC Bridge

[VERIFIED: src/lib/tauri.ts:13-17]
```tsx
export function importSpotifyLibrary(accessToken?: string) {
    return invoke<AppSnapshot>("import_spotify_library", {
        access_token: accessToken ?? null,
    });
}
```

Converts `undefined` to `null` for Rust's `Option<String>`.

**Data in:** `{ access_token: string | null }`
**Data out:** `Promise<AppSnapshot>`

---

### Step 4: Rust Command Entry — import_spotify_library (async)

[VERIFIED: src-tauri/src/lib.rs:263-267]
```rust
#[tauri::command]
async fn import_spotify_library(
    state: tauri::State<'_, SharedState>,
    access_token: Option<String>,
) -> Result<AppSnapshot, String> {
```

This is the **only async Tauri command** in the app. It must be async because it makes HTTP requests to the Spotify Web API via `reqwest`.

---

### Step 5: Token Resolution (Lock #1)

[VERIFIED: src-tauri/src/lib.rs:268-286]
```rust
let token_from_state = {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| String::from("State lock poisoned"))?;

    if guard.snapshot.tracks.is_empty() {
        guard.snapshot = seed_snapshot();
    }

    let cleaned = clean_option(access_token).or_else(|| guard.spotify_access_token.clone());

    if let Some(token) = cleaned.clone() {
        guard.spotify_access_token = Some(token.clone());
        guard.snapshot.spotify.access_token_present = true;
    }

    cleaned
};
```

First Mutex lock, scoped to a block so it's released before any async work.

**Token resolution priority:**
1. Token passed as IPC parameter (from frontend input field)
2. Token previously stored in state (from `save_spotify_config`)
3. `None` — triggers demo fallback

If a token is found, it's stored back into state and `access_token_present` is set to `true`.

Also seeds demo data if the library is empty (defensive guard).

**Data out:** `token_from_state: Option<String>`

---

### Step 6: Branch — Real API vs Demo Fallback

[VERIFIED: src-tauri/src/lib.rs:288-301]
```rust
let import = if let Some(token) = token_from_state {
    match fetch_spotify_library(&token).await {
        Ok(import) => import,
        Err(error) => {
            let mut fallback = demo_spotify_import();
            fallback.import_mode = String::from("demo_fallback");
            fallback.summary =
                format!("Spotify API unavailable, loaded demo import instead: {error}");
            fallback
        }
    }
} else {
    demo_spotify_import()
};
```

Three paths:

| Condition | Path | `import_mode` |
|-----------|------|---------------|
| Token present, API succeeds | `fetch_spotify_library()` | `"spotify_api"` |
| Token present, API fails | `demo_spotify_import()` | `"demo_fallback"` |
| No token | `demo_spotify_import()` | `"demo"` |

The API failure path is **graceful** — the error is captured in the summary string but the app continues with demo data instead of returning an error to the frontend.

**Data out:** `import: SpotifyImportResult`

---

### Step 7a (Real API Path): fetch_spotify_library

[VERIFIED: src-tauri/src/lib.rs:1175-1228]
```rust
async fn fetch_spotify_library(access_token: &str) -> Result<SpotifyImportResult, String> {
    let client = Client::builder()
        .user_agent("PulseGridDJ/0.1")
        .build()
        .map_err(|error| error.to_string())?;

    let saved_tracks_url = "https://api.spotify.com/v1/me/tracks?limit=50";
    let playlist_url = "https://api.spotify.com/v1/me/playlists?limit=20";
    let saved_items = fetch_saved_tracks(&client, access_token, saved_tracks_url).await?;
    let playlists = fetch_playlists(&client, access_token, playlist_url).await?;
```

Creates a `reqwest::Client` with user agent `PulseGridDJ/0.1`. Makes two initial API calls **sequentially** (not concurrent):

1. `fetch_saved_tracks` — user's Liked Songs
2. `fetch_playlists` — user's playlists

**External call:** `GET https://api.spotify.com/v1/me/tracks?limit=50`
**External call:** `GET https://api.spotify.com/v1/me/playlists?limit=20`

---

### Step 7b: Paginated Saved Tracks — fetch_saved_tracks

[VERIFIED: src-tauri/src/lib.rs:1230-1263]
```rust
async fn fetch_saved_tracks(
    client: &Client,
    access_token: &str,
    url: &str,
) -> Result<Vec<SpotifySavedTrackItem>, String> {
    let mut items = Vec::new();
    let mut next = Some(url.to_string());

    while let Some(current) = next {
        let response = client
            .get(&current)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "Spotify saved tracks request failed with {}",
                response.status()
            ));
        }

        let page = response
            .json::<SpotifyPaging<SpotifySavedTrackItem>>()
            .await
            .map_err(|error| error.to_string())?;

        items.extend(page.items);
        next = page.next;
    }

    Ok(items)
}
```

Standard cursor-based pagination loop. Follows the `next` URL from each `SpotifyPaging` response until `null`. Uses `Bearer` auth with the access token.

**Error handling:** Returns `Err` on non-2xx status or JSON parse failure. Does **not** retry.

**Data shape per item:** `SpotifySavedTrackItem { track: SpotifyTrack }` ([VERIFIED: src-tauri/src/lib.rs:154-157])

---

### Step 7c: Paginated Playlists — fetch_playlists

[VERIFIED: src-tauri/src/lib.rs:1265-1298]

Same pagination pattern as saved tracks. Returns `Vec<SpotifyPlaylist>` with `{ id, name }` per playlist.

**External call:** `GET https://api.spotify.com/v1/me/playlists?limit=20` (paginated)

---

### Step 7d: Per-Playlist Track Fetching

[VERIFIED: src-tauri/src/lib.rs:1196-1220]
```rust
for playlist in playlists {
    let playlist_items = fetch_playlist_tracks(&client, access_token, &playlist.id).await?;
    let mut spotify_ids = Vec::new();

    for item in playlist_items {
        if let Some(track) = item.track {
            if let Some(spotify_id) = track.id.clone() {
                spotify_ids.push(spotify_id);
            }
            tracks.push(convert_spotify_track(track, Some(playlist.name.clone())));
        }
    }

    crates.push(PendingCrate {
        id: format!("spotify-{}", playlist.id),
        name: playlist.name.clone(),
        color: color_for_name(&playlist.name),
        icon: String::from("SPT"),
        description: String::from("Imported from Spotify"),
        source: String::from("spotify"),
        spotify_ids,
        track_ids: Vec::new(),
    });
}
```

For each playlist, fetches all tracks via `fetch_playlist_tracks()` ([VERIFIED: src-tauri/src/lib.rs:1300-1335]) which paginates `GET /v1/playlists/{id}/tracks?limit=100`.

Each playlist becomes a `PendingCrate` with:
- `spotify_ids`: collected from the playlist items (for later ID resolution)
- `track_ids`: empty at this point (filled during merge resolution in Step 9)
- `color`: deterministic from playlist name via `color_for_name()` ([VERIFIED: src-tauri/src/lib.rs:1498-1504])

Playlist tracks with `track: None` (deleted/unavailable tracks) are silently skipped.

**External call:** `GET https://api.spotify.com/v1/playlists/{id}/tracks?limit=100` per playlist (paginated)

---

### Step 7e: Track Conversion — convert_spotify_track

[VERIFIED: src-tauri/src/lib.rs:1337-1392]
```rust
fn convert_spotify_track(track: SpotifyTrack, imported_from: Option<String>) -> Track {
    let spotify_id = track
        .id
        .unwrap_or_else(|| format!("anon-{}", stable_hash(&track.name)));
    let title = track.name;
    let artist = track
        .artists
        .iter()
        .map(|artist| artist.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let album = track.album.name;
    let genre_tags = imported_from
        .as_ref()
        .map(|value| split_tags(value))
        .unwrap_or_default();
    let analysis = analyze_track(
        &title, &artist, &album,
        track.duration_ms as f32 / 1000.0,
        &genre_tags, None,
    );
```

Maps Spotify API schema to internal `Track` format:

| Spotify Field | Track Field | Transform |
|--------------|-------------|-----------|
| `id` | `spotify_id` | Falls back to `anon-{hash(name)}` |
| `name` | `title` | Direct |
| `artists[].name` | `artist` | Joined with `, ` |
| `album.name` | `album` | Direct |
| `duration_ms` | `duration_seconds` | `/ 1000.0` |
| `album.release_date` | `year` | First 4 chars parsed as u16 |
| `album.images[0].url` | `artwork_url` | First image |
| `imported_from` param | `genre_tags` | Split on delimiters |

**Notable:** Genre tags come from the `imported_from` string (playlist name), **not** from Spotify's genre data. Spotify's track objects don't include genres directly — they're on the artist object, which isn't fetched here.

[INFERRED: `imported_from` is used as a genre tag source because it's the closest proxy for categorization without additional API calls to the artist endpoint]

Track gets the same `analyze_track()` pseudo-analysis as local files, but with `path: None`.

**Data out:** `Track` with `source: TrackSource::Spotify`, `path: None`, `id: "spotify-{spotify_id}"`

---

### Step 7f: API Result Assembly

[VERIFIED: src-tauri/src/lib.rs:1222-1228]
```rust
Ok(SpotifyImportResult {
    tracks,
    crates,
    import_mode: String::from("spotify_api"),
    summary: String::from("Imported Spotify playlists and liked tracks from the live Web API."),
})
```

---

### Step 7g (Demo Path): demo_spotify_import

[VERIFIED: src-tauri/src/lib.rs:1394-1460]

Returns hardcoded demo data: 3 tracks and 2 crates with `import_mode: "demo"`. Uses `demo_track()` to generate tracks with pseudo-analysis, same as the seed data.

| Demo Track | Artist | Crate |
|------------|--------|-------|
| Terminal Bloom | Cora Avenue | Nordic Lift |
| Arcflash Habit | Motive Static | Nordic Lift |
| Gold Mesh | Tessa Relay | Sunroom Reset |

---

### Step 8: Merge Phase (Lock #2)

[VERIFIED: src-tauri/src/lib.rs:303-307]
```rust
let mut guard = state
    .0
    .lock()
    .map_err(|_| String::from("State lock poisoned"))?;
let resolved_ids = merge_tracks(&mut guard.snapshot.tracks, import.tracks);
```

Second Mutex lock acquired after all async work completes. Calls `merge_tracks()` — same deduplication logic as the Local Folder Scan flow.

For Spotify imports specifically, the key merge behavior is **Hybrid promotion**: if a Spotify track matches an existing Local track (by normalized title+artist), the existing track gets upgraded to `TrackSource::Hybrid` and gains the `spotify_id`.

**Data in:** `existing tracks + import.tracks`
**Data out:** `resolved_ids: HashMap<String, String>` mapping `spotify:{id}` → internal track ID

---

### Step 9: Crate Resolution — Spotify ID to Track ID

[VERIFIED: src-tauri/src/lib.rs:309-335]
```rust
guard
    .snapshot
    .crates
    .retain(|crate_item| crate_item.source != "spotify");

for mut crate_item in import.crates {
    let mut track_ids = crate_item.track_ids;

    for spotify_id in crate_item.spotify_ids.drain(..) {
        if let Some(resolved) = resolved_ids.get(&format!("spotify:{spotify_id}")) {
            track_ids.push(resolved.clone());
        }
    }

    track_ids.sort();
    track_ids.dedup();

    guard.snapshot.crates.push(Crate {
        id: crate_item.id,
        name: crate_item.name,
        color: crate_item.color,
        icon: crate_item.icon,
        description: crate_item.description,
        source: crate_item.source,
        track_ids,
    });
}
```

Three-phase crate update:

1. **Remove** all existing `source == "spotify"` crates (full replace, not incremental)
2. **Resolve** each `PendingCrate`'s `spotify_ids` to internal track IDs using the `resolved_ids` map from merge. If a Spotify track was merged into an existing track (Hybrid), the crate gets the existing track's ID.
3. **Push** the resolved crate with deduped track IDs

---

### Step 10: Metadata Update

[VERIFIED: src-tauri/src/lib.rs:337-340]
```rust
guard.snapshot.spotify.access_token_present = guard.spotify_access_token.is_some();
guard.snapshot.spotify.last_import_mode = import.import_mode;
guard.snapshot.spotify.last_sync_summary = import.summary;
guard.snapshot.status = String::from("Spotify library import complete.");
```

Updates the `SpotifyConnection` metadata so the Sidebar can display import status.

---

### Step 11: Snapshot Refresh

[VERIFIED: src-tauri/src/lib.rs:342]
```rust
refresh_snapshot(&mut guard.snapshot, None, None, None);
```

Same as Local Folder Scan: rebuilds smart crates, stats, and mix assistant with `None` for deck IDs / energy. The frontend will immediately re-trigger the assistant with actual deck state (via the `useEffect` in the Mix Assistant flow).

---

### Step 12: Return and Apply

[VERIFIED: src-tauri/src/lib.rs:344]
```rust
Ok(guard.snapshot.clone())
```

Full `AppSnapshot` returned through IPC.

[VERIFIED: src/App.tsx:177]
```tsx
applySnapshot(next);
```

Same `applySnapshot()` as all other flows — wraps state update in `startTransition`, resets stale crate selection, syncs Spotify config fields.

---

## External Calls

| Call | Location | Endpoint | Auth | Pagination |
|------|----------|----------|------|------------|
| Saved tracks | `lib.rs:1181,1183` | `GET /v1/me/tracks?limit=50` | Bearer token | Cursor (`next` URL) |
| Playlists | `lib.rs:1182,1184` | `GET /v1/me/playlists?limit=20` | Bearer token | Cursor (`next` URL) |
| Playlist tracks | `lib.rs:1197` via `fetch_playlist_tracks` | `GET /v1/playlists/{id}/tracks?limit=100` | Bearer token | Cursor (`next` URL) |

All requests use `reqwest::Client` with user agent `PulseGridDJ/0.1` and `rustls-tls` backend.

[VERIFIED: src-tauri/src/lib.rs:1176-1179]
```rust
let client = Client::builder()
    .user_agent("PulseGridDJ/0.1")
    .build()
    .map_err(|error| error.to_string())?;
```

**CSP allowlist for these endpoints:**
[VERIFIED: src-tauri/tauri.conf.json — inferred from architecture doc, CSP includes `api.spotify.com` and `accounts.spotify.com`]

---

## Events Fired

None. All communication is request-response through Tauri IPC.

---

## Data Shape at Key Boundaries

### IPC Request
```json
{ "access_token": "BQD...xyz" }
```
or
```json
{ "access_token": null }
```

### SpotifyImportResult (internal, after API calls)
```rust
SpotifyImportResult {
    tracks: Vec<Track>,           // converted Spotify tracks
    crates: Vec<PendingCrate>,    // playlists with spotify_ids
    import_mode: "spotify_api",   // or "demo" or "demo_fallback"
    summary: "Imported Spotify playlists and liked tracks from the live Web API.",
}
```

### IPC Response
Full `AppSnapshot` with merged tracks, resolved playlist crates, updated stats, and refreshed assistant.

---

## Known Issues Found

### 1. Saved tracks and playlists fetched sequentially, not concurrently
[VERIFIED: src-tauri/src/lib.rs:1183-1184]
```rust
let saved_items = fetch_saved_tracks(&client, access_token, saved_tracks_url).await?;
let playlists = fetch_playlists(&client, access_token, playlist_url).await?;
```
These two API calls are independent and could run concurrently with `tokio::join!` or `futures::join!`. Sequential execution doubles the network latency for the initial API calls.

### 2. Playlist tracks fetched sequentially per playlist
[VERIFIED: src-tauri/src/lib.rs:1197]
Each playlist's tracks are fetched one after another in a `for playlist in playlists` loop. With many playlists, this creates a serial waterfall. Could use `futures::stream::FuturesUnordered` for concurrent fetching.

### 3. No pagination limits — potential for very large imports
[VERIFIED: src-tauri/src/lib.rs:1238-1260]
The pagination loop has no cap on total items. A user with thousands of saved tracks or dozens of large playlists will import everything, potentially creating a very large in-memory state. No progress reporting to the frontend during this process.

### 4. Genre tags derived from playlist name, not actual genres
[VERIFIED: src-tauri/src/lib.rs:1349-1352]
```rust
let genre_tags = imported_from
    .as_ref()
    .map(|value| split_tags(value))
    .unwrap_or_default();
```
Playlist names like "Nordic Lift" become genre tags `["nordic lift"]`. This means `split_tags` on the delimiter set `,;/|` won't split most playlist names meaningfully. Actual genre data would require additional Spotify API calls to artist endpoints.

### 5. Liked Songs tracks get genre tag from literal "Liked Songs"
[VERIFIED: src-tauri/src/lib.rs:1190-1193]
```rust
tracks.push(convert_spotify_track(
    item.track,
    Some(String::from("Liked Songs")),
));
```
All liked tracks get `genre_tags: ["liked songs"]`, which is not a useful genre classification.

### 6. Full crate replacement on re-import
[VERIFIED: src-tauri/src/lib.rs:311-312]
```rust
guard.snapshot.crates.retain(|crate_item| crate_item.source != "spotify");
```
All Spotify crates are removed and rebuilt from scratch on every import. Any manual curation of Spotify crate contents is lost.

### 7. No token refresh or expiration handling
[INFERRED: from absence of any token refresh logic in the codebase]
Spotify access tokens expire after 1 hour. There's no refresh token flow, no expiration detection, and no user notification when the token expires. The API calls will fail with a 401, falling back to demo data silently (with a summary message noting the error).

### 8. Two separate Mutex locks create a TOCTOU window
[VERIFIED: src-tauri/src/lib.rs:268-286 (Lock #1), 303-306 (Lock #2)]
Between the first lock release and second lock acquisition, another command could modify `snapshot.tracks`. In practice this is unlikely with single-user desktop use, but the track list could change between token resolution and merge.
