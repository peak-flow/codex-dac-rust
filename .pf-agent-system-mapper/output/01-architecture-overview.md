# PulseGrid DJ Architecture Overview

## Metadata
| Field | Value |
|-------|-------|
| Repository | `peak-codex-dac-rust` |
| Commit | N/A (not a git repository) |
| Documented | `2026-03-18` |
| Verification Status | `Verified` |

## Verification Summary
- [VERIFIED]: 52 claims
- [INFERRED]: 3 claims
- [NOT_FOUND]: 5 items (tests, database, routing library, state management library, CI/CD)
- [ASSUMED]: 1 item (Tauri asset protocol convention)

---

## 0. System Classification
| Field | Value |
|-------|-------|
| Category | Traditional Code |
| Type | Desktop Application (Tauri 2 + React SPA) |
| Evidence | `src-tauri/Cargo.toml` with `tauri = "2"`, `package.json` with `react`, `@tauri-apps/api`, `@tauri-apps/cli`; no web server routes; desktop window config in `tauri.conf.json` |
| Overlay Loaded | No |
| Confidence | `[VERIFIED]` |

## Example Reference
| Field | Value |
|-------|-------|
| Example Read | `.pf-agent-system-mapper/examples/react/good-architecture-doc-example.md` |
| Key Format Elements Noted | Tables for component map, verification tags on every claim, Section 3 uses tables not arrows, NOT_FOUND sections explicit |

---

## 1. System Purpose

PulseGrid DJ is a **desktop-first DJ application** that combines local music folder scanning, Spotify metadata import, a unified track library with dual decks, waveform-driven transport controls, and an algorithmic mix assistant that scores next-track suggestions by BPM, key compatibility, and energy targeting. It is built for DJs who want Serato/Pioneer-style workflows with an AI set-planning copilot.

[VERIFIED: `README.md:1-3`]
```
PulseGrid DJ is a desktop-first MVP DJ application built with Rust, Tauri, React, and TypeScript.
It combines local music folder scanning, Spotify metadata import, a unified library, dual decks,
waveform-driven transport controls, and an AI mix assistant inspired by Serato and Pioneer-style workflows.
```

---

## 2. Component Map

### Backend (Rust / Tauri)

| Component | Location | Responsibility | Verified |
|-----------|----------|----------------|----------|
| Application entry | `src-tauri/src/main.rs:1-3` | Calls `pulsegrid_dj::run()` | [VERIFIED] |
| Tauri builder & command registration | `src-tauri/src/lib.rs:1506-1526` | Configures plugins, state, invoke handler, window setup | [VERIFIED] |
| SharedState / PersistentState | `src-tauri/src/lib.rs:13-20` | `Mutex`-guarded in-memory app state holding `AppSnapshot` and Spotify token | [VERIFIED] |
| AppSnapshot | `src-tauri/src/lib.rs:22-31` | Top-level data model: tracks, crates, spotify, stats, assistant | [VERIFIED] |
| Track | `src-tauri/src/lib.rs:109-128` | Full track model with id, source, path, BPM, key, energy, waveform, cue points | [VERIFIED] |
| Crate | `src-tauri/src/lib.rs:77-86` | Named collection of track IDs with color/icon/source metadata | [VERIFIED] |
| `bootstrap_app` command | `src-tauri/src/lib.rs:205-217` | Seeds demo data on first call, returns snapshot | [VERIFIED] |
| `save_spotify_config` command | `src-tauri/src/lib.rs:219-261` | Stores Spotify client_id, redirect_uri, access_token in state | [VERIFIED] |
| `import_spotify_library` command | `src-tauri/src/lib.rs:263-345` | Fetches playlists and liked tracks from Spotify Web API or falls back to demo import | [VERIFIED] |
| `scan_music_folder` command | `src-tauri/src/lib.rs:347-402` | Walks a local directory, extracts audio metadata via `lofty`, merges into library | [VERIFIED] |
| `build_mix_assistant` command | `src-tauri/src/lib.rs:404-423` | Generates next-track suggestions scored by compatibility | [VERIFIED] |
| `scan_local_folder` | `src-tauri/src/lib.rs:720-750` | Walks directory tree for supported audio files (mp3, wav, flac, m4a, aac, ogg, aiff) | [VERIFIED] |
| `extract_local_track` | `src-tauri/src/lib.rs:752-810` | Reads tags via `lofty`, runs `analyze_track` to produce BPM/key/energy estimates | [VERIFIED] |
| `analyze_track` | `src-tauri/src/lib.rs:821-906` | Deterministic pseudo-analysis: derives BPM from genre tags, energy from BPM+tags, key from hash, waveform from seed | [VERIFIED] |
| `merge_tracks` | `src-tauri/src/lib.rs:938-1008` | Deduplicates by Spotify ID, path, or normalized title+artist; promotes to Hybrid on cross-source match | [VERIFIED] |
| `generate_mix_assistant` | `src-tauri/src/lib.rs:1023-1130` | Scores candidates by BPM distance, key compatibility, and target energy; returns top 5 suggestions with insights | [VERIFIED] |
| `key_compatible` | `src-tauri/src/lib.rs:1144-1166` | Camelot wheel compatibility: same number different letter, or adjacent numbers same letter | [VERIFIED] |
| `fetch_spotify_library` | `src-tauri/src/lib.rs:1175-1228` | Calls Spotify Web API for saved tracks and playlists, converts to internal Track format | [VERIFIED] |
| `demo_spotify_import` | `src-tauri/src/lib.rs:1394-1460` | Returns hardcoded demo Spotify tracks and crates for offline use | [VERIFIED] |
| `build_smart_crates` | `src-tauri/src/lib.rs:658-718` | Auto-generates 4 smart crates: Warmup Drift, Peak Voltage, Open Format Flex, After Hours | [VERIFIED] |
| Tauri permissions | `src-tauri/permissions/commands.toml` | Defines allow-list for all 5 Tauri commands | [VERIFIED] |
| Tauri capabilities | `src-tauri/capabilities/default.json` | Grants core:default, dialog:default, and custom command permissions to main window | [VERIFIED] |

### Frontend (React / TypeScript)

| Component | Location | Responsibility | Verified |
|-----------|----------|----------------|----------|
| React entry | `src/main.tsx:1-11` | Renders `<App />` inside `React.StrictMode` | [VERIFIED] |
| App | `src/App.tsx:44-417` | Root component: orchestrates decks, library, assistant, sidebar; manages snapshot state | [VERIFIED] |
| Sidebar | `src/components/Sidebar.tsx:11-119` | Displays stats, Spotify status, smart crates, imported crates | [VERIFIED] |
| DeckPanel | `src/components/DeckPanel.tsx:56-212` | Deck UI: waveform grid, transport controls, tempo/volume sliders, cue bank | [VERIFIED] |
| LibraryTable | `src/components/LibraryTable.tsx:29-113` | Sortable track table with load-to-deck buttons | [VERIFIED] |
| AssistantPanel | `src/components/AssistantPanel.tsx:11-89` | Mix assistant: suggested tracks with compatibility scores, set advice insights, energy target dial | [VERIFIED] |
| useDeck hook | `src/hooks/useDeck.ts:30-281` | Deck state machine: play/pause/stop/seek/eject, HTMLAudioElement integration, visual-mode fallback timer | [VERIFIED] |
| Tauri bridge | `src/lib/tauri.ts:1-33` | Typed wrappers around `invoke()` for all 5 backend commands | [VERIFIED] |
| Type definitions | `src/lib/types.ts:1-93` | TypeScript interfaces mirroring Rust structs: Track, Crate, AppSnapshot, etc. | [VERIFIED] |
| Global styles | `src/styles.css:1-650` | Dark-theme CSS with glass/blur aesthetics, grid layouts, responsive breakpoints | [VERIFIED] |

[NOT_FOUND: searched for "test", "spec", ".test.", ".spec." in project root and src/]
No test files found anywhere in the project.

[NOT_FOUND: searched for "redux", "zustand", "recoil", "jotai" in package.json and src/]
No external state management library. State is lifted via `useState` in App.tsx and passed as props.

---

## 3. Execution Surfaces & High-Level Data Movement (Discovery Only)

### 3.1 Primary Execution Surfaces

| Entry Surface | Type | Primary Components Involved | Evidence |
|--------------|------|-----------------------------|----------|
| Tauri window launch | Desktop App | `main.rs` -> `lib.rs::run()` -> Tauri Builder | [VERIFIED: `src-tauri/src/main.rs:1-3`, `src-tauri/src/lib.rs:1506-1526`] |
| `bootstrap_app` IPC command | Tauri invoke | SharedState, seed_snapshot, demo_tracks | [VERIFIED: `src-tauri/src/lib.rs:205-217`] |
| `scan_music_folder` IPC command | Tauri invoke | scan_local_folder, extract_local_track, merge_tracks, refresh_snapshot | [VERIFIED: `src-tauri/src/lib.rs:347-402`] |
| `save_spotify_config` IPC command | Tauri invoke | SharedState (stores credentials) | [VERIFIED: `src-tauri/src/lib.rs:219-261`] |
| `import_spotify_library` IPC command | Tauri invoke | fetch_spotify_library / demo_spotify_import, merge_tracks, refresh_snapshot | [VERIFIED: `src-tauri/src/lib.rs:263-345`] |
| `build_mix_assistant` IPC command | Tauri invoke | generate_mix_assistant, compatibility_score, key_compatible | [VERIFIED: `src-tauri/src/lib.rs:404-423`] |

### 3.2 High-Level Data Movement (Non-Procedural)

| Stage | Input Type | Output Type | Participating Components |
|------|------------|-------------|--------------------------|
| App Bootstrap | None | `AppSnapshot` with demo tracks | `bootstrap_app`, `seed_snapshot`, `demo_tracks` |
| Local Folder Scan | Filesystem path | Merged `AppSnapshot` with local tracks | `scan_music_folder`, `scan_local_folder`, `extract_local_track`, `lofty` (tag reader), `merge_tracks` |
| Spotify Import | Access token (optional) | Merged `AppSnapshot` with Spotify tracks and crates | `import_spotify_library`, `fetch_spotify_library` / `demo_spotify_import`, `merge_tracks` |
| Spotify Config | Client ID, redirect URI, token | Updated `AppSnapshot` | `save_spotify_config`, `SharedState` |
| Mix Assistant Generation | Deck track IDs, target energy | `MixAssistantPayload` with suggestions and insights | `build_mix_assistant`, `generate_mix_assistant`, `compatibility_score`, `key_compatible` |
| Frontend Render | `AppSnapshot` from backend | Visual UI (decks, library, sidebar, assistant) | App.tsx, Sidebar, DeckPanel, LibraryTable, AssistantPanel |
| Audio Playback | Local file path | HTMLAudioElement stream | `useDeck`, `convertFileSrc` (Tauri asset protocol) |

Detailed execution flows for each operation are candidates for **02-code-flows.md**.

### 3.3 Pointers to Code Flow Documentation

The following operations are candidates for **detailed flow tracing** (see 02-code-flows.md):

- **Local Folder Scan Flow** -- from directory picker through `walkdir` traversal, `lofty` tag extraction, `analyze_track` pseudo-analysis, `merge_tracks` deduplication, to `refresh_snapshot` with smart crate rebuilding
- **Spotify Import Flow** -- from token validation through paginated Spotify Web API calls, track conversion, dedup merge, crate resolution, to final snapshot refresh
- **Mix Assistant Scoring Flow** -- from anchor track selection through candidate filtering, `compatibility_score` calculation (BPM/key/energy weights), sorting and truncation, to insight generation
- **Deck Audio Playback Flow** -- from `loadTrack` through `convertFileSrc` asset resolution, HTMLAudioElement lifecycle, `useEffectEvent` sync, to visual-mode timer fallback for metadata-only tracks

---

## 3b. Frontend -> Backend Interaction Map

| Frontend Source | Trigger Type | Backend Target | Handler / Method | Evidence |
|-----------------|--------------|----------------|------------------|----------|
| `src/App.tsx` (useEffect on mount) | direct call | `src-tauri/src/lib.rs` | `bootstrap_app` | [VERIFIED: `src/App.tsx:80-102`, `src/lib/tauri.ts:5-7`] |
| `src/App.tsx` (handleChooseFolder) | direct call (after dialog) | `src-tauri/src/lib.rs` | `scan_music_folder` | [VERIFIED: `src/App.tsx:134-154`, `src/lib/tauri.ts:19-21`] |
| `src/App.tsx` (handleSaveSpotify) | direct call (button click) | `src-tauri/src/lib.rs` | `save_spotify_config` | [VERIFIED: `src/App.tsx:156-170`, `src/lib/tauri.ts:9-11`] |
| `src/App.tsx` (handleImportSpotify) | direct call (button click) | `src-tauri/src/lib.rs` | `import_spotify_library` | [VERIFIED: `src/App.tsx:172-182`, `src/lib/tauri.ts:13-17`] |
| `src/App.tsx` (useEffect on deck/energy change) | direct call | `src-tauri/src/lib.rs` | `build_mix_assistant` | [VERIFIED: `src/App.tsx:104-132`, `src/lib/tauri.ts:23-33`] |
| `src/App.tsx` (handleChooseFolder) | dialog open | Tauri dialog plugin | `open()` | [VERIFIED: `src/App.tsx:135-139`, `import { open } from "@tauri-apps/plugin-dialog"`] |

---

## 4. File/Folder Conventions

```
peak-codex-dac-rust/
├── index.html                    # Vite HTML entry, mounts /src/main.tsx [VERIFIED]
├── package.json                  # Node deps: React 19, Tauri API 2, Vite 7 [VERIFIED]
├── vite.config.ts                # Vite + React plugin, port 1420, TAURI_ env prefix [VERIFIED]
├── tsconfig.json                 # TypeScript config targeting ES2022, react-jsx [VERIFIED]
├── src/
│   ├── main.tsx                  # React root render [VERIFIED]
│   ├── App.tsx                   # Root component, all state orchestration [VERIFIED]
│   ├── styles.css                # Global dark-theme CSS [VERIFIED]
│   ├── vite-env.d.ts             # Vite client types reference [VERIFIED]
│   ├── components/
│   │   ├── AssistantPanel.tsx     # AI mix suggestions and insights [VERIFIED]
│   │   ├── DeckPanel.tsx          # Deck UI with waveform, transport, sliders [VERIFIED]
│   │   ├── LibraryTable.tsx       # Track browser table [VERIFIED]
│   │   └── Sidebar.tsx            # Stats, crate navigation, Spotify status [VERIFIED]
│   ├── hooks/
│   │   └── useDeck.ts             # Deck state + audio element management [VERIFIED]
│   └── lib/
│       ├── tauri.ts               # Typed invoke wrappers [VERIFIED]
│       └── types.ts               # TypeScript interfaces for Rust structs [VERIFIED]
├── src-tauri/
│   ├── Cargo.toml                 # Rust deps: tauri 2, lofty, reqwest, walkdir, serde [VERIFIED]
│   ├── tauri.conf.json            # App config: window size, CSP, asset protocol [VERIFIED]
│   ├── build.rs                   # Tauri build hook [VERIFIED]
│   ├── src/
│   │   ├── main.rs                # Entry: calls run() [VERIFIED]
│   │   └── lib.rs                 # ALL backend logic in single 1527-line file [VERIFIED]
│   ├── capabilities/
│   │   └── default.json           # Window permission grants [VERIFIED]
│   └── permissions/
│       ├── commands.toml           # Per-command allow-list definitions [VERIFIED]
│       └── default.toml            # Default permission set aggregating all commands [VERIFIED]
└── dist/                          # Vite build output [ASSUMED: standard Vite convention]
```

---

## 5. External Dependencies

### Rust Dependencies (Cargo.toml)

| Crate | Version | Purpose | Evidence |
|-------|---------|---------|----------|
| `tauri` | 2 | Desktop app framework, IPC, window management, asset protocol | [VERIFIED: `Cargo.toml:17`] |
| `tauri-plugin-dialog` | 2 | Native file/folder picker dialog | [VERIFIED: `Cargo.toml:18`] |
| `lofty` | 0.23 | Audio file metadata (ID3, Vorbis, etc.) tag reading | [VERIFIED: `Cargo.toml:14`] |
| `reqwest` | 0.12 | HTTP client for Spotify Web API calls (rustls-tls, json) | [VERIFIED: `Cargo.toml:15`] |
| `walkdir` | 2 | Recursive directory traversal for folder scanning | [VERIFIED: `Cargo.toml:19`] |
| `serde` | 1 | Serialization/deserialization for IPC data transfer | [VERIFIED: `Cargo.toml:16`] |

### Node Dependencies (package.json)

| Package | Version | Purpose | Evidence |
|---------|---------|---------|----------|
| `react` | ^19.0.0 | UI framework | [VERIFIED: `package.json:13`] |
| `react-dom` | ^19.0.0 | React DOM renderer | [VERIFIED: `package.json:14`] |
| `@tauri-apps/api` | ^2.0.0 | Tauri frontend IPC bridge (`invoke`, `convertFileSrc`) | [VERIFIED: `package.json:12`] |
| `@tauri-apps/plugin-dialog` | ^2.0.0 | Frontend dialog API for folder picker | [VERIFIED: `package.json:13`] |
| `vite` | ^7.0.0 | Dev server and build tool | [VERIFIED: `package.json:24`] |
| `@vitejs/plugin-react` | ^5.0.0 | React JSX transform for Vite | [VERIFIED: `package.json:21`] |
| `typescript` | ^5.7.0 | Type checking | [VERIFIED: `package.json:23`] |

### External Services

| Service | Endpoints Used | Configuration | Evidence |
|---------|---------------|---------------|----------|
| Spotify Web API | `https://api.spotify.com/v1/me/tracks`, `https://api.spotify.com/v1/me/playlists`, `https://api.spotify.com/v1/playlists/{id}/tracks` | CSP in `tauri.conf.json`, token stored in-memory via `save_spotify_config` | [VERIFIED: `lib.rs:1181-1183`, `tauri.conf.json:25`] |

[NOT_FOUND: searched for "database", "sqlite", "postgres", "mysql", "redis" in src-tauri/ and src/]
No database. All state is in-memory only; no persistence across sessions.

---

## 6. Known Issues & Risks

### 1. Single-File Backend (1527 lines)

All Rust backend logic is in `src-tauri/src/lib.rs` -- commands, data models, Spotify API client, audio scanning, mix assistant, demo data, and utility functions. This is a clear candidate for decomposition into modules.

[VERIFIED: `src-tauri/src/lib.rs` is 1527 lines containing 20+ structs, 5 commands, and ~30 functions]

### 2. No Persistent Storage

Application state exists only in-memory within `Mutex<PersistentState>`. Library data, scanned tracks, Spotify imports, and crate configurations are lost when the app closes.

[VERIFIED: `src-tauri/src/lib.rs:13-14`]
```rust
#[derive(Default)]
struct SharedState(Mutex<PersistentState>);
```

[NOT_FOUND: searched for "write", "save", "persist", "sqlite", "file::write" in lib.rs]
No file or database write operations exist.

### 3. Pseudo-Analysis Instead of Real Audio Analysis

BPM, key, energy, and waveform values are deterministically derived from metadata strings using a hash-based approach, not from actual audio signal processing.

[VERIFIED: `src-tauri/src/lib.rs:821-828`]
```rust
fn analyze_track(
    title: &str,
    artist: &str,
    album: &str,
    duration_seconds: f32,
    genre_tags: &[String],
    path: Option<&Path>,
) -> TrackAnalysis {
```

[INFERRED] The `stable_hash` function (FNV-1a variant at line 1487) is used as the seed for all "analysis" values, meaning identical metadata always produces identical BPM/key/energy regardless of actual audio content.

### 4. Spotify Token Security

The Spotify access token is stored as a plain `Option<String>` in memory and passed through IPC as a plaintext string. No encryption, no secure storage, no token refresh flow.

[VERIFIED: `src-tauri/src/lib.rs:19`]
```rust
spotify_access_token: Option<String>,
```

[VERIFIED: `src/App.tsx:59`]
```tsx
const [spotifyAccessToken, setSpotifyAccessToken] = useState("");
```

### 5. No Error Boundaries in React

Frontend catch blocks log to console and update a status line, but there are no React Error Boundaries to prevent full-app crashes from component render errors.

[VERIFIED: `src/App.tsx:93-95`]
```tsx
} catch (error) {
  console.error(error);
  setStatusLine("Unable to bootstrap the DJ library.");
}
```

[NOT_FOUND: searched for "ErrorBoundary", "componentDidCatch", "getDerivedStateFromError" in src/]
No error boundary components found.

### 6. No Tests

[NOT_FOUND: searched for "test", "spec", "#[test]", "#[cfg(test)]" in src-tauri/src/ and src/]
No Rust tests and no JavaScript/TypeScript test files exist in the project.

### 7. Hardcoded Demo Data

Eight demo tracks and two demo crates are hardcoded directly in `lib.rs`. Demo Spotify import also returns hardcoded tracks. This inflates the backend file and mixes sample data with production code.

[VERIFIED: `src-tauri/src/lib.rs:482-565` (demo_tracks), `src-tauri/src/lib.rs:1394-1460` (demo_spotify_import)]

### 8. CSP Uses unsafe-inline

The Content Security Policy allows `'unsafe-inline'` for both `style-src` and `script-src`, which weakens XSS protection.

[VERIFIED: `src-tauri/tauri.conf.json:25`]
```
style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline';
```

---

## 7. Entry Points Summary

| Route/Entry | Method | Handler | Middleware | Verified |
|-------------|--------|---------|------------|----------|
| App window launch | Tauri desktop | `main.rs` -> `lib.rs::run()` | None | [VERIFIED] |
| `bootstrap_app` | Tauri IPC invoke | `lib.rs::bootstrap_app()` | Tauri command permission (`allow-bootstrap-app`) | [VERIFIED] |
| `save_spotify_config` | Tauri IPC invoke | `lib.rs::save_spotify_config()` | Tauri command permission (`allow-save-spotify-config`) | [VERIFIED] |
| `import_spotify_library` | Tauri IPC invoke (async) | `lib.rs::import_spotify_library()` | Tauri command permission (`allow-import-spotify-library`) | [VERIFIED] |
| `scan_music_folder` | Tauri IPC invoke | `lib.rs::scan_music_folder()` | Tauri command permission (`allow-scan-music-folder`) | [VERIFIED] |
| `build_mix_assistant` | Tauri IPC invoke | `lib.rs::build_mix_assistant()` | Tauri command permission (`allow-build-mix-assistant`) | [VERIFIED] |

[NOT_FOUND: searched for "Route", "router", "react-router" in package.json and src/]
No client-side routing library. The app is a single-view desktop application with no URL-based navigation.

---

## 8. Technology Stack Summary

| Layer | Technology |
|-------|------------|
| Desktop Framework | Tauri 2 [VERIFIED: `Cargo.toml:17`] |
| Backend Language | Rust (edition 2024) [VERIFIED: `Cargo.toml:8`] |
| Frontend Framework | React 19 [VERIFIED: `package.json:13`] |
| Frontend Language | TypeScript 5.7+ [VERIFIED: `package.json:23`] |
| Build Tool (Frontend) | Vite 7 [VERIFIED: `package.json:24`] |
| Audio Metadata | lofty 0.23 [VERIFIED: `Cargo.toml:14`] |
| HTTP Client | reqwest 0.12 (rustls-tls) [VERIFIED: `Cargo.toml:15`] |
| File Traversal | walkdir 2 [VERIFIED: `Cargo.toml:19`] |
| Serialization | serde 1 [VERIFIED: `Cargo.toml:16`] |
| Styling | Plain CSS (dark theme, no framework) [VERIFIED: `src/styles.css`] |
| External Services | Spotify Web API (metadata import) [VERIFIED: `lib.rs:1181-1183`] |
| Database | None (in-memory only) [VERIFIED] |
| Audio Playback | HTMLAudioElement via Tauri asset protocol [VERIFIED: `src/hooks/useDeck.ts:120`] |

---

## What This System Does NOT Have

Based on searches finding no results:

1. **No Persistent Storage** -- in-memory state only, lost on close [VERIFIED]
2. **No Real Audio Analysis** -- BPM/key/energy are hash-derived estimates, not DSP [VERIFIED]
3. **No Tests** -- no Rust tests, no JS/TS test files [NOT_FOUND]
4. **No Database** -- no SQLite, PostgreSQL, or any storage engine [NOT_FOUND]
5. **No Client-Side Routing** -- single view, no react-router [NOT_FOUND]
6. **No CI/CD** -- no GitHub Actions, no Dockerfiles, no deployment config [NOT_FOUND: searched for ".github", "Dockerfile", "docker-compose" in project root]
7. **No OAuth Flow** -- Spotify token is pasted manually, no automated auth [INFERRED: from `save_spotify_config` accepting raw token string and README noting "Paste a Spotify user access token into the UI"]
8. **No Actual Audio Streaming for Spotify Tracks** -- Spotify imports are metadata-only; only local files with a filesystem path can produce audio [VERIFIED: `README.md:63-65`]
