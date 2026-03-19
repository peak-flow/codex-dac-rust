use lofty::file::{AudioFile, TaggedFileExt};
use lofty::read_from_path;
use lofty::tag::Accessor;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use regex::Regex;
use tauri::{Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use walkdir::WalkDir;

#[derive(Default)]
struct SharedState(Mutex<PersistentState>);

#[derive(Default, Clone)]
struct PersistentState {
    snapshot: AppSnapshot,
    spotify_access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppSnapshot {
    tracks: Vec<Track>,
    crates: Vec<Crate>,
    spotify: SpotifyConnection,
    stats: LibraryStats,
    assistant: MixAssistantPayload,
    last_scan_path: Option<String>,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpotifyConnection {
    client_id: Option<String>,
    redirect_uri: Option<String>,
    access_token_present: bool,
    last_sync_summary: String,
    last_import_mode: String,
    recommended_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LibraryStats {
    total_tracks: usize,
    local_tracks: usize,
    spotify_tracks: usize,
    hybrid_tracks: usize,
    avg_bpm: f32,
    avg_energy: f32,
    genres: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MixAssistantPayload {
    headline: String,
    summary: String,
    target_energy: f32,
    suggestions: Vec<SuggestionTrack>,
    insights: Vec<AssistantInsight>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SuggestionTrack {
    track_id: String,
    reason: String,
    compatibility: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AssistantInsight {
    title: String,
    detail: String,
    priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Crate {
    id: String,
    name: String,
    color: String,
    icon: String,
    description: String,
    source: String,
    track_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CuePoint {
    label: String,
    time_seconds: f32,
    color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TrackSource {
    Local,
    Spotify,
    Hybrid,
}

impl Default for TrackSource {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Track {
    id: String,
    source: TrackSource,
    path: Option<String>,
    spotify_id: Option<String>,
    title: String,
    artist: String,
    album: String,
    duration_seconds: f32,
    bpm: f32,
    musical_key: String,
    energy: f32,
    genre_tags: Vec<String>,
    waveform: Vec<f32>,
    cue_points: Vec<CuePoint>,
    imported_from: Option<String>,
    year: Option<u16>,
    artwork_url: Option<String>,
    stem_parent_id: Option<String>,
    stem_type: Option<String>,
    #[serde(default)]
    stem_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SpotifyConfigInput {
    client_id: Option<String>,
    redirect_uri: Option<String>,
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyPaging<T> {
    items: Vec<T>,
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotifyPlaylist {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct SpotifyPlaylistTrackItem {
    track: Option<SpotifyTrack>,
}

#[derive(Debug, Deserialize)]
struct SpotifySavedTrackItem {
    track: SpotifyTrack,
}

#[derive(Debug, Deserialize, Clone)]
struct SpotifyTrack {
    id: Option<String>,
    name: String,
    duration_ms: u32,
    album: SpotifyAlbum,
    artists: Vec<SpotifyArtist>,
}

#[derive(Debug, Deserialize, Clone)]
struct SpotifyAlbum {
    name: String,
    release_date: Option<String>,
    images: Option<Vec<SpotifyImage>>,
}

#[derive(Debug, Deserialize, Clone)]
struct SpotifyArtist {
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct SpotifyImage {
    url: String,
}

#[derive(Debug)]
struct SpotifyImportResult {
    tracks: Vec<Track>,
    crates: Vec<PendingCrate>,
    import_mode: String,
    summary: String,
}

#[derive(Debug, Clone)]
struct PendingCrate {
    id: String,
    name: String,
    color: String,
    icon: String,
    description: String,
    source: String,
    spotify_ids: Vec<String>,
    track_ids: Vec<String>,
}

#[tauri::command]
fn bootstrap_app(state: tauri::State<'_, SharedState>) -> Result<AppSnapshot, String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| String::from("State lock poisoned"))?;

    if guard.snapshot.tracks.is_empty() {
        guard.snapshot = seed_snapshot();
    }

    Ok(guard.snapshot.clone())
}

#[tauri::command]
fn save_spotify_config(
    state: tauri::State<'_, SharedState>,
    config: SpotifyConfigInput,
) -> Result<AppSnapshot, String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| String::from("State lock poisoned"))?;

    if guard.snapshot.tracks.is_empty() {
        guard.snapshot = seed_snapshot();
    }

    if let Some(client_id) = clean_option(config.client_id) {
        guard.snapshot.spotify.client_id = Some(client_id);
    }

    if let Some(redirect_uri) = clean_option(config.redirect_uri) {
        guard.snapshot.spotify.redirect_uri = Some(redirect_uri);
    }

    match clean_option(config.access_token) {
        Some(token) => {
            guard.spotify_access_token = Some(token);
            guard.snapshot.spotify.access_token_present = true;
            guard.snapshot.spotify.last_sync_summary = String::from(
                "Credentials staged. Import will pull playlists and liked songs with the supplied token.",
            );
        }
        None => {
            guard.spotify_access_token = None;
            guard.snapshot.spotify.access_token_present = false;
            guard.snapshot.spotify.last_sync_summary = String::from(
                "No token stored. Spotify import will use curated demo metadata until a user token is provided.",
            );
        }
    }

    guard.snapshot.status = String::from("Spotify settings updated.");

    Ok(guard.snapshot.clone())
}

#[tauri::command]
async fn import_spotify_library(
    state: tauri::State<'_, SharedState>,
    access_token: Option<String>,
) -> Result<AppSnapshot, String> {
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

    let mut guard = state
        .0
        .lock()
        .map_err(|_| String::from("State lock poisoned"))?;
    let resolved_ids = merge_tracks(&mut guard.snapshot.tracks, import.tracks);

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

    guard.snapshot.spotify.access_token_present = guard.spotify_access_token.is_some();
    guard.snapshot.spotify.last_import_mode = import.import_mode;
    guard.snapshot.spotify.last_sync_summary = import.summary;
    guard.snapshot.status = String::from("Spotify library import complete.");

    refresh_snapshot(&mut guard.snapshot, None, None, None);

    Ok(guard.snapshot.clone())
}

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
    let folder_name = folder
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Local Collection")
        .to_string();

    let mut guard = state
        .0
        .lock()
        .map_err(|_| String::from("State lock poisoned"))?;

    if guard.snapshot.tracks.is_empty() {
        guard.snapshot = seed_snapshot();
    }

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

    guard.snapshot.last_scan_path = Some(folder.to_string_lossy().to_string());
    guard.snapshot.status = format!("Scanned local library from {folder_name}.");

    refresh_snapshot(&mut guard.snapshot, None, None, None);

    Ok(guard.snapshot.clone())
}

#[tauri::command]
fn build_mix_assistant(
    state: tauri::State<'_, SharedState>,
    deck_a_track_id: Option<String>,
    deck_b_track_id: Option<String>,
    target_energy: Option<f32>,
) -> Result<MixAssistantPayload, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| String::from("State lock poisoned"))?;
    let payload = generate_mix_assistant(
        &guard.snapshot.tracks,
        deck_a_track_id.as_deref(),
        deck_b_track_id.as_deref(),
        target_energy,
    );

    Ok(payload)
}

fn seed_snapshot() -> AppSnapshot {
    let mut snapshot = AppSnapshot {
        tracks: demo_tracks(),
        crates: vec![
            Crate {
                id: String::from("resident-peak"),
                name: String::from("Resident Peak"),
                color: String::from("#ff7a18"),
                icon: String::from("PEAK"),
                description: String::from("High-pressure peak-time selections"),
                source: String::from("local"),
                track_ids: vec![
                    String::from("demo-cascade-nova"),
                    String::from("demo-solar-rush"),
                    String::from("demo-night-traction"),
                ],
            },
            Crate {
                id: String::from("spotlight-playlist"),
                name: String::from("Warmup Control"),
                color: String::from("#29f0b4"),
                icon: String::from("LIST"),
                description: String::from("A gentle lead-in for a rooftop or after-hours room"),
                source: String::from("spotify"),
                track_ids: vec![
                    String::from("demo-velvet-system"),
                    String::from("demo-copper-loop"),
                    String::from("demo-skyline-lift"),
                ],
            },
        ],
        spotify: SpotifyConnection {
            client_id: None,
            redirect_uri: Some(String::from("http://127.0.0.1:8888/callback")),
            access_token_present: false,
            last_sync_summary: String::from(
                "Demo Spotify metadata is loaded by default. Supply a user token to import real playlists and liked tracks.",
            ),
            last_import_mode: String::from("demo"),
            recommended_scopes: vec![
                String::from("playlist-read-private"),
                String::from("playlist-read-collaborative"),
                String::from("user-library-read"),
            ],
        },
        stats: LibraryStats::default(),
        assistant: MixAssistantPayload::default(),
        last_scan_path: None,
        status: String::from(
            "PulseGrid is live. Load tracks, scan a folder, or import Spotify metadata.",
        ),
    };

    refresh_snapshot(&mut snapshot, None, None, Some(0.72));
    snapshot
}

fn demo_tracks() -> Vec<Track> {
    vec![
        demo_track(
            "demo-cascade-nova",
            TrackSource::Hybrid,
            "Cascade Nova",
            "Eli Meridian",
            "Pressure Theatre",
            388.0,
            vec!["melodic house", "peak-time"],
            Some("Festival Ops"),
        ),
        demo_track(
            "demo-solar-rush",
            TrackSource::Spotify,
            "Solar Rush",
            "Mina Circuit",
            "Orbit Manual",
            342.0,
            vec!["progressive", "anthem"],
            Some("Sunrise Control"),
        ),
        demo_track(
            "demo-night-traction",
            TrackSource::Local,
            "Night Traction",
            "Vector Plaza",
            "Warehouse Drafts",
            417.0,
            vec!["tech house", "club"],
            Some("Basement Motion"),
        ),
        demo_track(
            "demo-velvet-system",
            TrackSource::Spotify,
            "Velvet System",
            "June Alloy",
            "Magnetic Avenue",
            321.0,
            vec!["warmup", "deep house"],
            Some("Warmup Control"),
        ),
        demo_track(
            "demo-copper-loop",
            TrackSource::Spotify,
            "Copper Loop",
            "Signal Belle",
            "Index Nights",
            298.0,
            vec!["afro house", "percussive"],
            Some("Warmup Control"),
        ),
        demo_track(
            "demo-skyline-lift",
            TrackSource::Hybrid,
            "Skyline Lift",
            "Atlas Process",
            "Mirrored District",
            365.0,
            vec!["house", "open format"],
            Some("Warmup Control"),
        ),
        demo_track(
            "demo-ghost-engine",
            TrackSource::Local,
            "Ghost Engine",
            "Rhea Current",
            "Zero Markers",
            291.0,
            vec!["breaks", "after-hours"],
            Some("Afterglow"),
        ),
        demo_track(
            "demo-lux-prism",
            TrackSource::Spotify,
            "Lux Prism",
            "Arden Flux",
            "Mirror Work",
            354.0,
            vec!["electro", "peak-time"],
            Some("Festival Ops"),
        ),
    ]
}

fn demo_track(
    id: &str,
    source: TrackSource,
    title: &str,
    artist: &str,
    album: &str,
    duration_seconds: f32,
    tags: Vec<&str>,
    imported_from: Option<&str>,
) -> Track {
    let tags = tags.into_iter().map(str::to_string).collect::<Vec<_>>();
    let analysis = analyze_track(title, artist, album, duration_seconds, &tags, None);

    Track {
        id: id.to_string(),
        source,
        path: None,
        spotify_id: id.strip_prefix("demo-").map(|value| value.to_string()),
        title: title.to_string(),
        artist: artist.to_string(),
        album: album.to_string(),
        duration_seconds,
        bpm: analysis.bpm,
        musical_key: analysis.musical_key,
        energy: analysis.energy,
        genre_tags: tags,
        waveform: analysis.waveform,
        cue_points: analysis.cue_points,
        imported_from: imported_from.map(str::to_string),
        year: Some(2024),
        artwork_url: None,
        stem_parent_id: None,
        stem_type: None,
        stem_ids: Vec::new(),
    }
}

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

fn build_stats(tracks: &[Track]) -> LibraryStats {
    let mut genres = HashSet::new();

    for track in tracks {
        for tag in &track.genre_tags {
            genres.insert(tag.to_lowercase());
        }
    }

    let total = tracks.len().max(1) as f32;

    LibraryStats {
        total_tracks: tracks.len(),
        local_tracks: tracks
            .iter()
            .filter(|track| matches!(track.source, TrackSource::Local))
            .count(),
        spotify_tracks: tracks
            .iter()
            .filter(|track| matches!(track.source, TrackSource::Spotify))
            .count(),
        hybrid_tracks: tracks
            .iter()
            .filter(|track| matches!(track.source, TrackSource::Hybrid))
            .count(),
        avg_bpm: tracks.iter().map(|track| track.bpm).sum::<f32>() / total,
        avg_energy: tracks.iter().map(|track| track.energy).sum::<f32>() / total,
        genres: genres.len(),
    }
}

fn build_smart_crates(tracks: &[Track]) -> Vec<Crate> {
    vec![
        Crate {
            id: String::from("smart-warmup"),
            name: String::from("Warmup Drift"),
            color: String::from("#29f0b4"),
            icon: String::from("WRM"),
            description: String::from("Lower-pressure entries with friendly blend points"),
            source: String::from("smart"),
            track_ids: tracks
                .iter()
                .filter(|track| track.energy < 0.62)
                .map(|track| track.id.clone())
                .collect(),
        },
        Crate {
            id: String::from("smart-peak"),
            name: String::from("Peak Voltage"),
            color: String::from("#ff7a18"),
            icon: String::from("PK"),
            description: String::from("Higher-energy tracks for the middle of the room"),
            source: String::from("smart"),
            track_ids: tracks
                .iter()
                .filter(|track| track.energy >= 0.74)
                .map(|track| track.id.clone())
                .collect(),
        },
        Crate {
            id: String::from("smart-open"),
            name: String::from("Open Format Flex"),
            color: String::from("#ffcd6f"),
            icon: String::from("OF"),
            description: String::from("Tracks suited for fast pivots and crowd recovery"),
            source: String::from("smart"),
            track_ids: tracks
                .iter()
                .filter(|track| {
                    track
                        .genre_tags
                        .iter()
                        .any(|tag| tag.contains("open") || tag.contains("house"))
                })
                .map(|track| track.id.clone())
                .collect(),
        },
        Crate {
            id: String::from("smart-after"),
            name: String::from("After Hours"),
            color: String::from("#88a8ff"),
            icon: String::from("AH"),
            description: String::from("Cool-down or late-room textures for long blends"),
            source: String::from("smart"),
            track_ids: tracks
                .iter()
                .filter(|track| track.energy < 0.72 && track.bpm < 124.0)
                .map(|track| track.id.clone())
                .collect(),
        },
    ]
}

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
        stem_parent_id: None,
        stem_type: None,
        stem_ids: Vec::new(),
    }
}

#[derive(Debug)]
struct TrackAnalysis {
    bpm: f32,
    musical_key: String,
    energy: f32,
    waveform: Vec<f32>,
    cue_points: Vec<CuePoint>,
}

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
        title,
        artist,
        album,
        duration_seconds,
        path.map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    );
    let seed = stable_hash(&source);
    let tags = genre_tags
        .iter()
        .map(|tag| tag.to_lowercase())
        .collect::<Vec<_>>();

    let bpm = if tags
        .iter()
        .any(|tag| tag.contains("drum") || tag.contains("jungle"))
    {
        168.0 + (seed % 8) as f32
    } else if tags
        .iter()
        .any(|tag| tag.contains("hip hop") || tag.contains("trap"))
    {
        76.0 + (seed % 18) as f32
    } else if tags.iter().any(|tag| tag.contains("afro")) {
        116.0 + (seed % 8) as f32
    } else if tags
        .iter()
        .any(|tag| tag.contains("tech") || tag.contains("house"))
    {
        122.0 + (seed % 8) as f32
    } else if tags
        .iter()
        .any(|tag| tag.contains("progressive") || tag.contains("anthem"))
    {
        126.0 + (seed % 6) as f32
    } else {
        100.0 + (seed % 36) as f32
    };

    let energy_bias = if tags
        .iter()
        .any(|tag| tag.contains("peak") || tag.contains("anthem") || tag.contains("club"))
    {
        0.18
    } else if tags
        .iter()
        .any(|tag| tag.contains("warmup") || tag.contains("deep") || tag.contains("after"))
    {
        -0.12
    } else {
        0.0
    };

    let energy =
        (((bpm / 180.0) * 0.65) + ((seed % 24) as f32 / 100.0) + energy_bias).clamp(0.42, 0.98);
    let keys = [
        "1A", "2A", "3A", "4A", "5A", "6A", "7A", "8A", "9A", "10A", "11A", "12A", "1B", "2B",
        "3B", "4B", "5B", "6B", "7B", "8B", "9B", "10B", "11B", "12B",
    ];
    let key_index = ((seed as usize) + title.len() + artist.len() + album.len()) % keys.len();
    let waveform = (0..72)
        .map(|index| {
            let phase = (seed as f32 / 100.0) + index as f32 * 0.37;
            let pulse = ((phase.sin() + 1.0) * 0.35) + (((phase * 0.47).cos() + 1.0) * 0.15);
            (pulse + ((seed % (index as u64 + 7)) as f32 / 90.0)).clamp(0.08, 1.0)
        })
        .collect::<Vec<_>>();

    TrackAnalysis {
        bpm,
        musical_key: keys[key_index].to_string(),
        energy,
        waveform,
        cue_points: generate_cue_points(duration_seconds.max(180.0)),
    }
}

fn generate_cue_points(duration_seconds: f32) -> Vec<CuePoint> {
    vec![
        CuePoint {
            label: String::from("Intro"),
            time_seconds: 0.0,
            color: String::from("#ffcd6f"),
        },
        CuePoint {
            label: String::from("Blend"),
            time_seconds: (duration_seconds * 0.18).round(),
            color: String::from("#29f0b4"),
        },
        CuePoint {
            label: String::from("Drop"),
            time_seconds: (duration_seconds * 0.34).round(),
            color: String::from("#ff7a18"),
        },
        CuePoint {
            label: String::from("Break"),
            time_seconds: (duration_seconds * 0.57).round(),
            color: String::from("#88a8ff"),
        },
        CuePoint {
            label: String::from("Outro"),
            time_seconds: (duration_seconds * 0.82).round(),
            color: String::from("#ff4858"),
        },
    ]
}

fn merge_tracks(existing: &mut Vec<Track>, incoming: Vec<Track>) -> HashMap<String, String> {
    let mut resolved = HashMap::new();

    for mut candidate in incoming {
        if let Some(existing_track) = existing
            .iter_mut()
            .find(|track| tracks_match(track, &candidate))
        {
            if existing_track.path.is_none() && candidate.path.is_some() {
                existing_track.path = candidate.path.take();
            }

            if existing_track.spotify_id.is_none() && candidate.spotify_id.is_some() {
                existing_track.spotify_id = candidate.spotify_id.take();
            }

            if matches!(existing_track.source, TrackSource::Local)
                && matches!(candidate.source, TrackSource::Spotify)
            {
                existing_track.source = TrackSource::Hybrid;
            }

            if matches!(existing_track.source, TrackSource::Spotify)
                && matches!(candidate.source, TrackSource::Local)
            {
                existing_track.source = TrackSource::Hybrid;
            }

            if existing_track.imported_from.is_none() {
                existing_track.imported_from = candidate.imported_from.take();
            }

            existing_track.genre_tags.extend(candidate.genre_tags);
            existing_track.genre_tags.sort();
            existing_track.genre_tags.dedup();
            existing_track.waveform = candidate.waveform;
            existing_track.cue_points = candidate.cue_points;
            existing_track.duration_seconds = existing_track
                .duration_seconds
                .max(candidate.duration_seconds);

            resolved.insert(
                format!("id:{}", existing_track.id),
                existing_track.id.clone(),
            );

            if let Some(path) = &existing_track.path {
                resolved.insert(format!("path:{path}"), existing_track.id.clone());
            }

            if let Some(spotify_id) = &existing_track.spotify_id {
                resolved.insert(format!("spotify:{spotify_id}"), existing_track.id.clone());
            }

            continue;
        }

        if let Some(path) = &candidate.path {
            resolved.insert(format!("path:{path}"), candidate.id.clone());
        }

        if let Some(spotify_id) = &candidate.spotify_id {
            resolved.insert(format!("spotify:{spotify_id}"), candidate.id.clone());
        }

        resolved.insert(format!("id:{}", candidate.id), candidate.id.clone());
        existing.push(candidate);
    }

    resolved
}

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

fn generate_mix_assistant(
    tracks: &[Track],
    deck_a_track_id: Option<&str>,
    deck_b_track_id: Option<&str>,
    target_energy: Option<f32>,
) -> MixAssistantPayload {
    if tracks.is_empty() {
        return MixAssistantPayload {
            headline: String::from("Load tracks to start building transitions."),
            summary: String::from("The assistant needs a library before it can shape a set."),
            target_energy: target_energy.unwrap_or(0.7),
            suggestions: Vec::new(),
            insights: Vec::new(),
        };
    }

    let deck_a = deck_a_track_id.and_then(|id| tracks.iter().find(|track| track.id == id));
    let deck_b = deck_b_track_id.and_then(|id| tracks.iter().find(|track| track.id == id));
    let anchor = deck_b.or(deck_a).unwrap_or(&tracks[0]);
    let target_energy = target_energy.unwrap_or(anchor.energy);

    let mut scored = tracks
        .iter()
        .filter(|track| track.id != anchor.id)
        .map(|track| {
            let compatibility = compatibility_score(anchor, track, target_energy);

            let reason = if key_compatible(&anchor.musical_key, &track.musical_key)
                && (anchor.bpm - track.bpm).abs() < 4.0
            {
                String::from("Strong harmonic blend and tight tempo handoff")
            } else if (anchor.energy - track.energy).abs() < 0.08 {
                String::from("Energy stays consistent while refreshing the palette")
            } else if track.energy > anchor.energy {
                String::from("Natural lift for a bigger room response")
            } else {
                String::from("Recovery option for breathing room between peaks")
            };

            SuggestionTrack {
                track_id: track.id.clone(),
                reason,
                compatibility,
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .compatibility
            .partial_cmp(&left.compatibility)
            .unwrap_or(Ordering::Equal)
    });
    scored.truncate(5);

    let mut insights = vec![AssistantInsight {
        title: String::from("Energy arc"),
        detail: format!(
            "Current anchor is {:.0}% energy. Targeting {:.0}% keeps the room {}.",
            anchor.energy * 100.0,
            target_energy * 100.0,
            if target_energy >= anchor.energy {
                "climbing"
            } else {
                "breathing"
            }
        ),
        priority: String::from("high"),
    }];

    if let Some(deck_a_track) = deck_a {
        if let Some(deck_b_track) = deck_b {
            insights.push(AssistantInsight {
                title: String::from("Deck relationship"),
                detail: format!(
                    "Deck A {} ({:.1} BPM / {}) and Deck B {} ({:.1} BPM / {}) are {:.1} BPM apart.",
                    deck_a_track.title,
                    deck_a_track.bpm,
                    deck_a_track.musical_key,
                    deck_b_track.title,
                    deck_b_track.bpm,
                    deck_b_track.musical_key,
                    (deck_a_track.bpm - deck_b_track.bpm).abs()
                ),
                priority: String::from("medium"),
            });
        }
    }

    insights.push(AssistantInsight {
        title: String::from("Best cue strategy"),
        detail: String::from(
            "Aim the next mix around the Blend cue, then commit on the Drop marker if the room wants acceleration.",
        ),
        priority: String::from("low"),
    });

    MixAssistantPayload {
        headline: format!("Best next move after {}", anchor.title),
        summary: format!(
            "Scored {} library tracks by BPM distance, key compatibility, and target energy.",
            tracks.len().saturating_sub(1)
        ),
        target_energy,
        suggestions: scored,
        insights,
    }
}

fn compatibility_score(anchor: &Track, candidate: &Track, target_energy: f32) -> f32 {
    let bpm_score = 1.0 - ((anchor.bpm - candidate.bpm).abs() / 12.0).clamp(0.0, 1.0);
    let key_score = if key_compatible(&anchor.musical_key, &candidate.musical_key) {
        1.0
    } else {
        0.42
    };
    let energy_score = 1.0 - ((candidate.energy - target_energy).abs() / 0.35).clamp(0.0, 1.0);

    ((bpm_score * 0.36) + (key_score * 0.34) + (energy_score * 0.30)).clamp(0.0, 1.0)
}

fn key_compatible(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    let Some((left_number, left_letter)) = split_key(left) else {
        return false;
    };
    let Some((right_number, right_letter)) = split_key(right) else {
        return false;
    };

    if left_number == right_number && left_letter != right_letter {
        return true;
    }

    if left_letter == right_letter {
        let diff = left_number.abs_diff(right_number);
        return diff == 1 || diff == 11;
    }

    false
}

fn split_key(value: &str) -> Option<(u8, char)> {
    let split_at = value.len().checked_sub(1)?;
    let (number, letter) = value.split_at(split_at);

    Some((number.parse().ok()?, letter.chars().next()?))
}

async fn fetch_spotify_library(access_token: &str) -> Result<SpotifyImportResult, String> {
    let client = Client::builder()
        .user_agent("PulseGridDJ/0.1")
        .build()
        .map_err(|error| error.to_string())?;

    let saved_tracks_url = "https://api.spotify.com/v1/me/tracks?limit=50";
    let playlist_url = "https://api.spotify.com/v1/me/playlists?limit=20";
    let saved_items = fetch_saved_tracks(&client, access_token, saved_tracks_url).await?;
    let playlists = fetch_playlists(&client, access_token, playlist_url).await?;

    let mut tracks = Vec::new();
    let mut crates = Vec::new();

    for item in saved_items {
        tracks.push(convert_spotify_track(
            item.track,
            Some(String::from("Liked Songs")),
        ));
    }

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

    Ok(SpotifyImportResult {
        tracks,
        crates,
        import_mode: String::from("spotify_api"),
        summary: String::from("Imported Spotify playlists and liked tracks from the live Web API."),
    })
}

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

async fn fetch_playlists(
    client: &Client,
    access_token: &str,
    url: &str,
) -> Result<Vec<SpotifyPlaylist>, String> {
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
                "Spotify playlist request failed with {}",
                response.status()
            ));
        }

        let page = response
            .json::<SpotifyPaging<SpotifyPlaylist>>()
            .await
            .map_err(|error| error.to_string())?;

        items.extend(page.items);
        next = page.next;
    }

    Ok(items)
}

async fn fetch_playlist_tracks(
    client: &Client,
    access_token: &str,
    playlist_id: &str,
) -> Result<Vec<SpotifyPlaylistTrackItem>, String> {
    let mut items = Vec::new();
    let mut next = Some(format!(
        "https://api.spotify.com/v1/playlists/{playlist_id}/tracks?limit=100"
    ));

    while let Some(current) = next {
        let response = client
            .get(&current)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        if !response.status().is_success() {
            return Err(format!(
                "Spotify playlist items request failed with {}",
                response.status()
            ));
        }

        let page = response
            .json::<SpotifyPaging<SpotifyPlaylistTrackItem>>()
            .await
            .map_err(|error| error.to_string())?;

        items.extend(page.items);
        next = page.next;
    }

    Ok(items)
}

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
        &title,
        &artist,
        &album,
        track.duration_ms as f32 / 1000.0,
        &genre_tags,
        None,
    );
    let year = track
        .album
        .release_date
        .as_deref()
        .and_then(|value| value.get(0..4))
        .and_then(|value| value.parse::<u16>().ok());

    Track {
        id: format!("spotify-{}", spotify_id),
        source: TrackSource::Spotify,
        path: None,
        spotify_id: Some(spotify_id),
        title,
        artist,
        album,
        duration_seconds: track.duration_ms as f32 / 1000.0,
        bpm: analysis.bpm,
        musical_key: analysis.musical_key,
        energy: analysis.energy,
        genre_tags,
        waveform: analysis.waveform,
        cue_points: analysis.cue_points,
        imported_from,
        year,
        artwork_url: track
            .album
            .images
            .as_ref()
            .and_then(|images| images.first())
            .map(|image| image.url.clone()),
        stem_parent_id: None,
        stem_type: None,
        stem_ids: Vec::new(),
    }
}

fn demo_spotify_import() -> SpotifyImportResult {
    let tracks = vec![
        demo_track(
            "spotify-demo-terminal",
            TrackSource::Spotify,
            "Terminal Bloom",
            "Cora Avenue",
            "Signal Flowers",
            314.0,
            vec!["deep house", "warmup"],
            Some("Nordic Lift"),
        ),
        demo_track(
            "spotify-demo-arcflash",
            TrackSource::Spotify,
            "Arcflash Habit",
            "Motive Static",
            "Wide Grid",
            372.0,
            vec!["progressive", "peak-time"],
            Some("Nordic Lift"),
        ),
        demo_track(
            "spotify-demo-goldmesh",
            TrackSource::Spotify,
            "Gold Mesh",
            "Tessa Relay",
            "After Image",
            287.0,
            vec!["afro house", "open format"],
            Some("Sunroom Reset"),
        ),
    ];

    SpotifyImportResult {
        tracks,
        crates: vec![
            PendingCrate {
                id: String::from("spotify-demo-nordic"),
                name: String::from("Nordic Lift"),
                color: String::from("#29f0b4"),
                icon: String::from("SPT"),
                description: String::from("Demo playlist import"),
                source: String::from("spotify"),
                spotify_ids: vec![String::from("terminal"), String::from("arcflash")],
                track_ids: vec![
                    String::from("spotify-demo-terminal"),
                    String::from("spotify-demo-arcflash"),
                ],
            },
            PendingCrate {
                id: String::from("spotify-demo-sunroom"),
                name: String::from("Sunroom Reset"),
                color: String::from("#ffcd6f"),
                icon: String::from("SPT"),
                description: String::from("Demo playlist import"),
                source: String::from("spotify"),
                spotify_ids: vec![String::from("goldmesh")],
                track_ids: vec![String::from("spotify-demo-goldmesh")],
            },
        ],
        import_mode: String::from("demo"),
        summary: String::from(
            "Loaded curated demo Spotify metadata. Supply a user access token to hit the live Web API.",
        ),
    }
}

fn clean_option(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn split_tags(value: impl AsRef<str>) -> Vec<String> {
    value
        .as_ref()
        .split(&[',', ';', '/', '|'][..])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn normalize_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric() || character.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn stable_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;

    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
}

fn color_for_name(name: &str) -> String {
    const COLORS: [&str; 6] = [
        "#29f0b4", "#ff7a18", "#ffcd6f", "#88a8ff", "#ff7aa6", "#96ff8e",
    ];
    let index = (stable_hash(name) as usize) % COLORS.len();
    COLORS[index].to_string()
}

#[derive(Clone, Serialize)]
struct StemProgressEvent {
    track_id: String,
    percent: f32,
    stage: String,
}

#[tauri::command]
async fn check_stems_ready() -> Result<bool, String> {
    let output = tokio::process::Command::new("uv")
        .args(["run", "--with", "demucs", "python", "-c", "import demucs; print('ok')"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await;

    match output {
        Ok(result) => Ok(result.status.success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
async fn separate_stems(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    track_id: String,
    stem_count: u8,
) -> Result<AppSnapshot, String> {
    if stem_count != 2 && stem_count != 4 {
        return Err(String::from("stem_count must be 2 or 4"));
    }

    let (track_path, track_title) = {
        let guard = state
            .0
            .lock()
            .map_err(|_| String::from("State lock poisoned"))?;

        let track = guard
            .snapshot
            .tracks
            .iter()
            .find(|track| track.id == track_id)
            .ok_or_else(|| String::from("Track not found in library"))?;

        if track.stem_parent_id.is_some() {
            return Err(String::from("Cannot separate a track that is already a stem"));
        }

        let path = track
            .path
            .as_ref()
            .ok_or_else(|| String::from("Track has no local file path. Only local audio files can be separated."))?
            .clone();

        (path, track.title.clone())
    };

    let source_path = PathBuf::from(&track_path);
    if !source_path.exists() {
        return Err(String::from("Source audio file no longer exists on disk"));
    }

    let source_dir = source_path
        .parent()
        .ok_or_else(|| String::from("Cannot determine parent directory of track"))?;
    let file_stem = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| String::from("Cannot determine file name"))?
        .to_string();

    let stems_out_dir = source_dir.join("stems");
    let expected_dir = stems_out_dir.join("htdemucs").join(&file_stem);

    let expected_files: Vec<&str> = if stem_count == 2 {
        vec!["vocals.wav", "no_vocals.wav"]
    } else {
        vec!["vocals.wav", "drums.wav", "bass.wav", "other.wav"]
    };

    let stems_cached = expected_dir.exists()
        && expected_files
            .iter()
            .all(|name| expected_dir.join(name).exists());

    if !stems_cached {
        app.emit(
            "stem-progress",
            StemProgressEvent {
                track_id: track_id.clone(),
                percent: 0.0,
                stage: String::from("running"),
            },
        )
        .ok();

        let mut cmd = tokio::process::Command::new("uv");
        cmd.args([
            "run", "--with", "demucs", "--with", "torch",
            "python", "-m", "demucs",
            "--out",
        ]);
        cmd.arg(stems_out_dir.as_os_str());

        if stem_count == 2 {
            cmd.arg("--two-stems=vocals");
        }

        cmd.arg(&track_path);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn().map_err(|error| {
            format!("Failed to spawn demucs. Is uv installed? Error: {error}")
        })?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| String::from("Failed to capture demucs stderr"))?;

        let progress_re = Regex::new(r"(\d+)%").unwrap();
        let progress_track_id = track_id.clone();
        let progress_app = app.clone();

        let reader_handle = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(captures) = progress_re.captures(&line) {
                    if let Ok(percent) = captures[1].parse::<f32>() {
                        progress_app
                            .emit(
                                "stem-progress",
                                StemProgressEvent {
                                    track_id: progress_track_id.clone(),
                                    percent,
                                    stage: String::from("running"),
                                },
                            )
                            .ok();
                    }
                }
            }
        });

        let exit_status = child
            .wait()
            .await
            .map_err(|error| format!("Demucs process error: {error}"))?;

        let _ = reader_handle.await;

        if !exit_status.success() {
            app.emit(
                "stem-progress",
                StemProgressEvent {
                    track_id: track_id.clone(),
                    percent: 0.0,
                    stage: String::from("error"),
                },
            )
            .ok();

            return Err(format!(
                "Demucs exited with code {}",
                exit_status.code().unwrap_or(-1)
            ));
        }
    }

    if !expected_dir.exists() {
        return Err(String::from(
            "Demucs completed but output directory was not created",
        ));
    }

    let mut stem_tracks = Vec::new();

    for stem_file in &expected_files {
        let stem_path = expected_dir.join(stem_file);

        if !stem_path.exists() {
            continue;
        }

        let stem_name = stem_file.trim_end_matches(".wav");
        let stem_label = match stem_name {
            "no_vocals" => String::from("Instrumental"),
            other => {
                let mut chars = other.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => other.to_string(),
                }
            }
        };

        let mut stem_track = extract_local_track(&stem_path);
        stem_track.id = format!(
            "stem-{}-{}",
            stable_hash(&stem_path.to_string_lossy()),
            stem_name
        );
        stem_track.title = format!("{} [{}]", track_title, stem_label);
        stem_track.stem_parent_id = Some(track_id.clone());
        stem_track.stem_type = Some(stem_name.to_string());
        stem_track.source = TrackSource::Local;

        stem_tracks.push(stem_track);
    }

    if stem_tracks.is_empty() {
        return Err(String::from(
            "No stem files were found after separation",
        ));
    }

    let stem_track_ids = stem_tracks
        .iter()
        .map(|track| track.id.clone())
        .collect::<Vec<_>>();

    let mut guard = state
        .0
        .lock()
        .map_err(|_| String::from("State lock poisoned"))?;

    if let Some(parent) = guard
        .snapshot
        .tracks
        .iter_mut()
        .find(|track| track.id == track_id)
    {
        parent.stem_ids = stem_track_ids;
    }

    for stem in stem_tracks {
        if !guard
            .snapshot
            .tracks
            .iter()
            .any(|existing| existing.id == stem.id)
        {
            guard.snapshot.tracks.push(stem);
        }
    }

    guard.snapshot.status = format!("Stems separated for {track_title}.");
    refresh_snapshot(&mut guard.snapshot, None, None, None);

    app.emit(
        "stem-progress",
        StemProgressEvent {
            track_id: track_id.clone(),
            percent: 100.0,
            stage: String::from("complete"),
        },
    )
    .ok();

    Ok(guard.snapshot.clone())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(SharedState::default())
        .invoke_handler(tauri::generate_handler![
            bootstrap_app,
            save_spotify_config,
            import_spotify_library,
            scan_music_folder,
            build_mix_assistant,
            check_stems_ready,
            separate_stems
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_title("PulseGrid DJ")?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running PulseGrid DJ");
}
