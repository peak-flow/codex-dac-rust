# Local Folder Scan Code Flow

## Metadata
| Field | Value |
|-------|-------|
| Repository | `peak-codex-dac-rust` |
| Commit | N/A (not a git repository) |
| Documented | `2026-03-18` |
| Trigger | User clicks "Scan Music Folder" button |
| End State | `AppSnapshot` returned with scanned tracks merged into library, folder crate created, smart crates rebuilt, stats recalculated |

## Verification Summary
- [VERIFIED]: 24
- [INFERRED]: 2
- [NOT_FOUND]: 0
- [ASSUMED]: 1

---

## Flow Diagram

```
[User clicks "Scan Music Folder" button]
        │
        ▼
  App.tsx::handleChooseFolder()
        │
        ├──→ open() (tauri-plugin-dialog)
        │        │
        │        ▼
        │    [Native OS folder picker]
        │        │
        │        ▼
        │    Returns path: string | null
        │
        ├──→ scanMusicFolder(path)  [IPC invoke]
        │        │
        │        ▼
        │    lib.rs::scan_music_folder()
        │        │
        │        ├──→ PathBuf::from() + exists() check
        │        │
        │        ├──→ scan_local_folder(&folder)
        │        │        │
        │        │        ├──→ WalkDir::new(folder) [recursive traversal]
        │        │        │
        │        │        └──→ extract_local_track(path) [per file]
        │        │                 │
        │        │                 ├──→ lofty::read_from_path()
        │        │                 │        │
        │        │                 │        ▼
        │        │                 │    [title, artist, album, duration, genre]
        │        │                 │
        │        │                 └──→ analyze_track()
        │        │                          │
        │        │                          ├──→ stable_hash() [FNV-1a seed]
        │        │                          ├──→ BPM from genre tag matching
        │        │                          ├──→ Energy from BPM + tag bias
        │        │                          ├──→ Key from Camelot wheel index
        │        │                          ├──→ Waveform from sine formula
        │        │                          └──→ generate_cue_points()
        │        │
        │        ├──→ merge_tracks(&mut tracks, scanned_tracks)
        │        │        │
        │        │        └──→ tracks_match() [dedup by spotify_id, path, or title+artist]
        │        │
        │        ├──→ Create folder Crate
        │        │
        │        └──→ refresh_snapshot()
        │                 │
        │                 ├──→ build_smart_crates()
        │                 ├──→ build_stats()
        │                 └──→ generate_mix_assistant()
        │
        ▼
  applySnapshot(next)  [React state update]
```

---

## Detailed Flow

### Step 1: Button Click — UI Entry Point

[VERIFIED: src/App.tsx:265-266]
```tsx
<button className="primary-action" onClick={() => void handleChooseFolder()} type="button">
    Scan Music Folder
</button>
```

The "Scan Music Folder" button in the Library Ingest control card triggers `handleChooseFolder()`.

**Calls:** `handleChooseFolder()` in App.tsx

---

### Step 2: Native Folder Picker Dialog

[VERIFIED: src/App.tsx:134-143]
```tsx
async function handleChooseFolder() {
    const picked = await open({
        directory: true,
        multiple: false,
        title: "Choose a music folder",
    });

    if (typeof picked !== "string") {
        return;
    }
```

**Data in:** None (user selects folder via OS-native dialog)
**Data out:** `picked: string` (absolute folder path) or `null` (user cancelled)

The `open()` function is from `@tauri-apps/plugin-dialog` ([VERIFIED: src/App.tsx:1]). It opens a native OS folder picker with `directory: true`, `multiple: false`. If the user cancels or picks nothing, the function returns early.

**Calls:** `scanMusicFolder(picked)` via Tauri IPC

---

### Step 3: Status Update + IPC Invoke

[VERIFIED: src/App.tsx:145-153]
```tsx
    setStatusLine(`Scanning ${picked} ...`);

    try {
        const next = await scanMusicFolder(picked);
        applySnapshot(next);
    } catch (error) {
        console.error(error);
        setStatusLine("Folder scan failed.");
    }
}
```

Sets a scanning status message, then calls `scanMusicFolder(picked)`.

**Data in:** `picked: string` (folder path)
**Data out:** `AppSnapshot` on success, error string on failure

---

### Step 4: Tauri IPC Bridge

[VERIFIED: src/lib/tauri.ts:19-21]
```tsx
export function scanMusicFolder(path: string) {
    return invoke<AppSnapshot>("scan_music_folder", { path });
}
```

Thin typed wrapper around Tauri's `invoke()`. Sends `{ path }` payload to the `scan_music_folder` Rust command. The IPC serialization is handled by Tauri's serde bridge.

**Data in:** `{ path: string }`
**Data out:** `Promise<AppSnapshot>`

---

### Step 5: Rust Command Entry — scan_music_folder

[VERIFIED: src-tauri/src/lib.rs:347-358]
```rust
#[tauri::command]
fn scan_music_folder(
    state: tauri::State<'_, SharedState>,
    path: String,
) -> Result<AppSnapshot, String> {
    let folder = PathBuf::from(path.trim());

    if !folder.exists() {
        return Err(String::from("Selected folder does not exist"));
    }

    let scanned_tracks = scan_local_folder(&folder)?;
```

The `#[tauri::command]` attribute registers this as a synchronous Tauri IPC handler. It validates the folder path exists, then delegates to `scan_local_folder()`.

**Data in:** `path: String` (from IPC), `state: SharedState` (Tauri-managed)
**Data out:** `Result<AppSnapshot, String>`
**Calls:** `scan_local_folder(&folder)`

[VERIFIED: src-tauri/src/lib.rs:1510-1514 — command registration]
```rust
.invoke_handler(tauri::generate_handler![
    bootstrap_app,
    save_spotify_config,
    import_spotify_library,
    scan_music_folder,
    build_mix_assistant
])
```

---

### Step 6: Directory Traversal — scan_local_folder

[VERIFIED: src-tauri/src/lib.rs:720-750]
```rust
fn scan_local_folder(folder: &Path) -> Result<Vec<Track>, String> {
    let supported = ["mp3", "wav", "flac", "m4a", "aac", "ogg", "aiff"];
    let mut tracks = Vec::new();

    for entry in WalkDir::new(folder)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let Some(extension) = entry.path().extension().and_then(|value| value.to_str()) else {
            continue;
        };

        if !supported
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(extension))
        {
            continue;
        }

        tracks.push(extract_local_track(entry.path()));
    }

    if tracks.is_empty() {
        return Err(String::from(
            "No supported audio files found in the selected folder",
        ));
    }

    Ok(tracks)
}
```

Uses `walkdir` crate for recursive directory traversal. Filters to files only, then matches extension case-insensitively against 7 supported audio formats. Returns an error if no audio files are found.

**Data in:** `folder: &Path` (validated directory)
**Data out:** `Result<Vec<Track>, String>` (one Track per audio file)
**Calls:** `extract_local_track(entry.path())` for each matching file

---

### Step 7: Audio Metadata Extraction — extract_local_track

[VERIFIED: src-tauri/src/lib.rs:752-810]
```rust
fn extract_local_track(path: &Path) -> Track {
    let file_name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Unknown Track");
    let mut title = file_name.replace('_', " ");
    let mut artist = String::from("Unknown Artist");
    let mut album = String::from("Local Library");
    let mut duration = 0.0_f32;
    let mut genre_tags = Vec::new();

    if let Ok(tagged_file) = read_from_path(path) {
        duration = tagged_file.properties().duration().as_secs_f32();

        if let Some(tag) = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())
        {
            if let Some(value) = tag.title() {
                title = value.to_string();
            }
            if let Some(value) = tag.artist() {
                artist = value.to_string();
            }
            if let Some(value) = tag.album() {
                album = value.to_string();
            }
            if let Some(value) = tag.genre() {
                genre_tags.extend(split_tags(value.as_ref()));
            }
        }
    }

    let analysis = analyze_track(&title, &artist, &album, duration, &genre_tags, Some(path));
    let path_string = path.to_string_lossy().to_string();

    Track {
        id: format!("local-{}", stable_hash(&path_string)),
        source: TrackSource::Local,
        path: Some(path_string),
        spotify_id: None,
        title,
        artist,
        album,
        duration_seconds: duration.max(1.0),
        bpm: analysis.bpm,
        musical_key: analysis.musical_key,
        energy: analysis.energy,
        genre_tags,
        waveform: analysis.waveform,
        cue_points: analysis.cue_points,
        imported_from: None,
        year: None,
        artwork_url: None,
    }
}
```

Initializes defaults from the filename (underscores replaced with spaces). Attempts to read audio tags using `lofty::read_from_path()` ([VERIFIED: src-tauri/src/lib.rs:1-3]). Falls back gracefully if tag reading fails — the track still gets created with file-derived defaults.

**Tag fallback chain:** `primary_tag()` -> `first_tag()` -> defaults

**Data in:** `path: &Path` (single audio file)
**Data out:** `Track` with source=`Local`, path set, id=`local-{hash}`

**Key details:**
- Track ID: `format!("local-{}", stable_hash(&path_string))` — deterministic from file path
- Duration: clamped to minimum 1.0 seconds
- Genre: split on `,;/|` delimiters via `split_tags()` ([VERIFIED: src-tauri/src/lib.rs:1469-1477])

**Calls:** `analyze_track()` for BPM/key/energy/waveform/cues

---

### Step 8: Pseudo-Analysis Pipeline — analyze_track

[VERIFIED: src-tauri/src/lib.rs:821-906]
```rust
fn analyze_track(
    title: &str,
    artist: &str,
    album: &str,
    duration_seconds: f32,
    genre_tags: &[String],
    path: Option<&Path>,
) -> TrackAnalysis {
    let source = format!(
        "{}|{}|{}|{}|{}",
        title, artist, album, duration_seconds,
        path.map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    let seed = stable_hash(&source);
```

All "analysis" is deterministic pseudo-analysis. A single FNV-1a hash seed is derived from concatenated metadata (`title|artist|album|duration|path`). This seed drives all computed values.

#### 8a: BPM Derivation

[VERIFIED: src-tauri/src/lib.rs:844-868]
```rust
let bpm = if tags.iter().any(|tag| tag.contains("drum") || tag.contains("jungle")) {
    168.0 + (seed % 8) as f32
} else if tags.iter().any(|tag| tag.contains("hip hop") || tag.contains("trap")) {
    76.0 + (seed % 18) as f32
} else if tags.iter().any(|tag| tag.contains("afro")) {
    116.0 + (seed % 8) as f32
} else if tags.iter().any(|tag| tag.contains("tech") || tag.contains("house")) {
    122.0 + (seed % 8) as f32
} else if tags.iter().any(|tag| tag.contains("progressive") || tag.contains("anthem")) {
    126.0 + (seed % 6) as f32
} else {
    100.0 + (seed % 36) as f32
};
```

BPM is genre-biased: each genre family has a base BPM + small random offset from the hash seed. Untagged tracks get 100-136 BPM.

| Genre Match | BPM Range |
|-------------|-----------|
| drum/jungle | 168-175 |
| hip hop/trap | 76-93 |
| afro | 116-123 |
| tech/house | 122-129 |
| progressive/anthem | 126-131 |
| (default) | 100-135 |

#### 8b: Energy Derivation

[VERIFIED: src-tauri/src/lib.rs:870-885]
```rust
let energy_bias = if tags.iter().any(|tag| tag.contains("peak") || tag.contains("anthem") || tag.contains("club")) {
    0.18
} else if tags.iter().any(|tag| tag.contains("warmup") || tag.contains("deep") || tag.contains("after")) {
    -0.12
} else {
    0.0
};

let energy = (((bpm / 180.0) * 0.65) + ((seed % 24) as f32 / 100.0) + energy_bias).clamp(0.42, 0.98);
```

Energy = `(BPM/180 * 0.65) + (hash jitter 0-0.23) + genre bias`. Clamped to [0.42, 0.98]. High-energy tags boost +0.18, warmup/deep tags reduce -0.12.

#### 8c: Key Derivation

[VERIFIED: src-tauri/src/lib.rs:886-890]
```rust
let keys = [
    "1A", "2A", "3A", "4A", "5A", "6A", "7A", "8A", "9A", "10A", "11A", "12A",
    "1B", "2B", "3B", "4B", "5B", "6B", "7B", "8B", "9B", "10B", "11B", "12B",
];
let key_index = ((seed as usize) + title.len() + artist.len() + album.len()) % keys.len();
```

Musical key is selected from the 24-position Camelot wheel by `(seed + title.len + artist.len + album.len) % 24`.

#### 8d: Waveform Generation

[VERIFIED: src-tauri/src/lib.rs:891-897]
```rust
let waveform = (0..72)
    .map(|index| {
        let phase = (seed as f32 / 100.0) + index as f32 * 0.37;
        let pulse = ((phase.sin() + 1.0) * 0.35) + (((phase * 0.47).cos() + 1.0) * 0.15);
        (pulse + ((seed % (index as u64 + 7)) as f32 / 90.0)).clamp(0.08, 1.0)
    })
    .collect::<Vec<_>>();
```

72-bar waveform from overlapping sine/cosine waves seeded by the hash. Clamped to [0.08, 1.0].

#### 8e: Cue Point Generation

[VERIFIED: src-tauri/src/lib.rs:908-936]
```rust
fn generate_cue_points(duration_seconds: f32) -> Vec<CuePoint> {
    vec![
        CuePoint { label: "Intro",  time_seconds: 0.0,                           color: "#ffcd6f" },
        CuePoint { label: "Blend",  time_seconds: (duration_seconds * 0.18).round(), color: "#29f0b4" },
        CuePoint { label: "Drop",   time_seconds: (duration_seconds * 0.34).round(), color: "#ff7a18" },
        CuePoint { label: "Break",  time_seconds: (duration_seconds * 0.57).round(), color: "#88a8ff" },
        CuePoint { label: "Outro",  time_seconds: (duration_seconds * 0.82).round(), color: "#ff4858" },
    ]
}
```

5 fixed cue points at proportional positions. Duration is clamped to minimum 180s before cue generation ([VERIFIED: src-tauri/src/lib.rs:904]).

**Data in:** `title, artist, album, duration, genre_tags, path`
**Data out:** `TrackAnalysis { bpm, musical_key, energy, waveform: Vec<f32>[72], cue_points: Vec<CuePoint>[5] }`

---

### Step 9: Track Deduplication — merge_tracks

[VERIFIED: src-tauri/src/lib.rs:374]
```rust
let resolved = merge_tracks(&mut guard.snapshot.tracks, scanned_tracks);
```

[VERIFIED: src-tauri/src/lib.rs:938-1008]
```rust
fn merge_tracks(existing: &mut Vec<Track>, incoming: Vec<Track>) -> HashMap<String, String> {
    let mut resolved = HashMap::new();

    for mut candidate in incoming {
        if let Some(existing_track) = existing
            .iter_mut()
            .find(|track| tracks_match(track, &candidate))
        {
```

For each incoming track, checks if it already exists in the library using `tracks_match()`.

**Match predicate** ([VERIFIED: src-tauri/src/lib.rs:1010-1021]):
```rust
fn tracks_match(left: &Track, right: &Track) -> bool {
    if left.spotify_id.is_some() && left.spotify_id == right.spotify_id {
        return true;
    }
    if left.path.is_some() && left.path == right.path {
        return true;
    }
    normalize_text(&left.title) == normalize_text(&right.title)
        && normalize_text(&left.artist) == normalize_text(&right.artist)
}
```

Three match strategies in priority order:
1. Same `spotify_id` (both non-None)
2. Same `path` (both non-None)
3. Same normalized `title` + `artist` (strips non-alphanumeric, lowercases)

**On match** ([VERIFIED: src-tauri/src/lib.rs:946-992]):
- Fills missing `path` or `spotify_id` from the incoming track
- Promotes source to `Hybrid` if Local+Spotify cross
- Merges genre tags (extend + sort + dedup)
- Overwrites waveform and cue_points from new analysis
- Takes the longer duration
- Builds resolved ID map entries for `id:`, `path:`, `spotify:` keys

**On no match** ([VERIFIED: src-tauri/src/lib.rs:994-1004]):
- Pushes the new track into the existing vector
- Adds resolved ID map entries

**Data in:** `existing: &mut Vec<Track>` (library), `incoming: Vec<Track>` (scanned)
**Data out:** `HashMap<String, String>` — maps `path:{path}` -> track ID for crate building

---

### Step 10: Folder Crate Creation

[VERIFIED: src-tauri/src/lib.rs:375-394]
```rust
let resolved = merge_tracks(&mut guard.snapshot.tracks, scanned_tracks);
let local_track_ids = resolved
    .iter()
    .filter_map(|(key, value)| key.starts_with("path:").then_some(value.clone()))
    .collect::<Vec<_>>();
let crate_id = format!("folder-{}", stable_hash(&folder.to_string_lossy()));

guard
    .snapshot
    .crates
    .retain(|crate_item| !(crate_item.source == "local" && crate_item.id == crate_id));

guard.snapshot.crates.push(Crate {
    id: crate_id,
    name: folder_name.clone(),
    color: String::from("#8bc5ff"),
    icon: String::from("DIR"),
    description: format!("Scanned from {folder_name}"),
    source: String::from("local"),
    track_ids: local_track_ids,
});
```

Creates a folder crate from the resolved ID map. The crate ID is deterministic (`folder-{hash_of_path}`), so re-scanning the same folder replaces the previous crate (retain + push pattern). Only `path:` keyed entries are included — ensuring only local tracks from this scan are in the crate.

**Data in:** `resolved: HashMap<String, String>`, folder path
**Data out:** New `Crate` appended to `snapshot.crates`

---

### Step 11: Snapshot Update Metadata

[VERIFIED: src-tauri/src/lib.rs:396-397]
```rust
guard.snapshot.last_scan_path = Some(folder.to_string_lossy().to_string());
guard.snapshot.status = format!("Scanned local library from {folder_name}.");
```

Records the last scanned folder path and updates the status line.

---

### Step 12: Snapshot Refresh — refresh_snapshot

[VERIFIED: src-tauri/src/lib.rs:399]
```rust
refresh_snapshot(&mut guard.snapshot, None, None, None);
```

[VERIFIED: src-tauri/src/lib.rs:601-625]
```rust
fn refresh_snapshot(
    snapshot: &mut AppSnapshot,
    deck_a_track_id: Option<&str>,
    deck_b_track_id: Option<&str>,
    target_energy: Option<f32>,
) {
    let manual_crates = snapshot
        .crates
        .iter()
        .filter(|crate_item| crate_item.source != "smart")
        .cloned()
        .collect::<Vec<_>>();

    let mut crates = manual_crates;
    crates.extend(build_smart_crates(&snapshot.tracks));

    snapshot.crates = crates;
    snapshot.stats = build_stats(&snapshot.tracks);
    snapshot.assistant = generate_mix_assistant(
        &snapshot.tracks,
        deck_a_track_id,
        deck_b_track_id,
        target_energy,
    );
}
```

Called with `None` for all deck/energy params (scan doesn't know deck state). Three sub-operations:

#### 12a: Smart Crate Rebuilding

[VERIFIED: src-tauri/src/lib.rs:658-718]

Replaces all `source == "smart"` crates with 4 auto-generated crates:

| Smart Crate | Filter Criteria |
|-------------|----------------|
| Warmup Drift | `energy < 0.62` |
| Peak Voltage | `energy >= 0.74` |
| Open Format Flex | genre tag contains "open" or "house" |
| After Hours | `energy < 0.72 && bpm < 124.0` |

#### 12b: Stats Rebuilding

[VERIFIED: src-tauri/src/lib.rs:627-656]

Computes: total_tracks, local/spotify/hybrid counts, avg_bpm, avg_energy, genre count (unique lowercased genre tags).

#### 12c: Mix Assistant Rebuild

[VERIFIED: src-tauri/src/lib.rs:619-624]

Called with `None` deck IDs and `None` energy target. Uses the first track as anchor and defaults to its energy as target. [INFERRED: with None params, `generate_mix_assistant` at line 1041 falls through to `&tracks[0]` as anchor]

---

### Step 13: Return Snapshot via IPC

[VERIFIED: src-tauri/src/lib.rs:401]
```rust
Ok(guard.snapshot.clone())
```

Clones the entire `AppSnapshot` and returns it through Tauri's IPC serialization (serde JSON). The Mutex guard is dropped here.

**Data out:**
```
AppSnapshot {
    tracks: Vec<Track>,         // existing + newly merged local tracks
    crates: Vec<Crate>,         // manual crates + folder crate + 4 smart crates
    spotify: SpotifyConnection, // unchanged
    stats: LibraryStats,        // recalculated
    assistant: MixAssistantPayload, // rebuilt with default anchor
    last_scan_path: Some("/path/to/scanned/folder"),
    status: "Scanned local library from FolderName."
}
```

---

### Step 14: Frontend State Application — applySnapshot

[VERIFIED: src/App.tsx:61-78]
```tsx
function applySnapshot(nextSnapshot: AppSnapshot) {
    startTransition(() => {
        setSnapshot(nextSnapshot);
        setStatusLine(nextSnapshot.status);
    });

    if (!nextSnapshot.crates.some((crateItem) => crateItem.id === selectedCrateId) && selectedCrateId !== "all") {
        setSelectedCrateId("all");
    }

    if (nextSnapshot.spotify.client_id) {
        setSpotifyClientId(nextSnapshot.spotify.client_id);
    }
    if (nextSnapshot.spotify.redirect_uri) {
        setSpotifyRedirectUri(nextSnapshot.spotify.redirect_uri);
    }
}
```

Wraps snapshot + status update in `startTransition` for non-blocking rendering. Resets crate selection to "all" if the previously selected crate no longer exists. Syncs Spotify config fields from the snapshot.

[INFERRED: `startTransition` is React 19's concurrency API — allows the snapshot update to be interrupted by higher-priority updates like user input]

**Data in:** `AppSnapshot` from backend
**Data out:** React state updates trigger re-render of Sidebar (stats, crates), LibraryTable (tracks), AssistantPanel (suggestions)

---

### Step 15: Automatic Mix Assistant Refresh (Post-Render Side Effect)

[VERIFIED: src/App.tsx:104-132]
```tsx
useEffect(() => {
    if (!snapshot) {
        return;
    }

    let cancelled = false;

    void (async () => {
        try {
            const assistant = await buildMixAssistant(
                deckA.state.track?.id,
                deckB.state.track?.id,
                targetEnergy,
            );

            if (cancelled) {
                return;
            }

            setSnapshot((current) => (current ? { ...current, assistant } : current));
        } catch (error) {
            console.error(error);
        }
    })();

    return () => {
        cancelled = true;
    };
}, [snapshot?.tracks.length, deckA.state.track?.id, deckB.state.track?.id, targetEnergy]);
```

After the scan snapshot is applied, `snapshot.tracks.length` changes, triggering this effect. It calls `build_mix_assistant` with the current deck state and energy target — giving a more accurate assistant than the one built during `refresh_snapshot` (which had `None` for deck IDs). Uses a `cancelled` flag for stale-closure protection.

---

## External Calls

| Call | Location | Details |
|------|----------|---------|
| Native OS folder picker | `src/App.tsx:135-139` via `@tauri-apps/plugin-dialog::open()` | Opens OS-native folder selection dialog. No network I/O. |
| Filesystem read (walkdir) | `src-tauri/src/lib.rs:724-726` | Recursive directory traversal. No network I/O. |
| Audio file read (lofty) | `src-tauri/src/lib.rs:763` via `read_from_path()` | Reads audio file headers/tags. No network I/O. |

No HTTP requests, no database queries, no external API calls in this flow.

---

## Events Fired

No events are dispatched in this flow. All communication is synchronous request-response through Tauri IPC. The frontend `useEffect` at `App.tsx:104` is triggered by React state change, not by an explicit event.

---

## Data Shape at Key Boundaries

### IPC Request (Frontend -> Backend)
```json
{ "path": "/Users/dj/Music/House" }
```

### Per-Track After Extraction (Backend Internal)
```rust
Track {
    id: "local-14928371049283",       // "local-" + FNV-1a hash of path
    source: TrackSource::Local,
    path: Some("/Users/dj/Music/House/track.mp3"),
    spotify_id: None,
    title: "Track Name",             // from ID3 tag or filename
    artist: "Artist Name",           // from tag or "Unknown Artist"
    album: "Album Name",             // from tag or "Local Library"
    duration_seconds: 342.0,         // from audio properties, min 1.0
    bpm: 124.0,                      // genre-biased pseudo-analysis
    musical_key: "8A",               // Camelot wheel from hash
    energy: 0.72,                    // BPM + hash + genre bias
    genre_tags: ["tech house"],      // from genre tag, split on delimiters
    waveform: [0.45, 0.62, ...],     // 72 floats from sine formula
    cue_points: [CuePoint x 5],      // proportional positions
    imported_from: None,
    year: None,                      // lofty doesn't extract year here
    artwork_url: None,
}
```

### IPC Response (Backend -> Frontend)
Full `AppSnapshot` with all tracks, crates, stats, and assistant — see Step 13.

---

## Known Issues Found

### 1. Year never extracted from local files
[VERIFIED: src-tauri/src/lib.rs:807] `year: None` is hardcoded. The `lofty` crate supports year/date tags, but `extract_local_track` doesn't read them.

### 2. Artwork never extracted from local files
[VERIFIED: src-tauri/src/lib.rs:808] `artwork_url: None` is hardcoded. Embedded cover art in audio files is ignored.

### 3. Tag read failure is silent
[VERIFIED: src-tauri/src/lib.rs:763] `if let Ok(tagged_file) = read_from_path(path)` — if tag reading fails (corrupted file, unsupported codec), the track is still created with filename-derived title, "Unknown Artist", zero duration. No warning or logging.

### 4. Scan is synchronous on the Rust side
[VERIFIED: src-tauri/src/lib.rs:348] `fn scan_music_folder` — not `async fn`. For large folders, this blocks the Tauri command thread. The frontend remains responsive (IPC is async from JS side), but no other Tauri commands can be processed until the scan completes.
[ASSUMED: Tauri 2 runs sync commands on a thread pool, so the main thread isn't blocked, but the specific command slot is occupied]

### 5. Re-scan doesn't remove stale tracks
[VERIFIED: src-tauri/src/lib.rs:374] `merge_tracks` only adds or updates tracks — it never removes tracks that were previously scanned but no longer exist on disk. Deleted files remain as phantom entries in the library.

### 6. Duplicate-scan produces duplicate cue points/waveform overwrites
[VERIFIED: src-tauri/src/lib.rs:973-974] On match, `existing_track.waveform = candidate.waveform` and `existing_track.cue_points = candidate.cue_points` — the incoming analysis always overwrites, even though it's deterministic and produces the same values. This is harmless but wasteful.

### 7. No progress reporting
The frontend shows "Scanning {path} ..." but has no way to know how many files have been processed or how many remain. The entire scan is one atomic IPC call with no intermediate updates.
