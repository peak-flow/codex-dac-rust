# Code Flow Recommendations: PulseGrid DJ

> Generated: 2026-03-18
> Based on: `.pf-agent-system-mapper/output/01-architecture-overview.md`

## Summary

| Flow | Priority | Components | Effort |
|------|----------|------------|--------|
| Local Folder Scan | High (12/12) | 7 | Medium |
| Mix Assistant Scoring | High (10/12) | 4 | Low |
| Spotify Import | High (10/12) | 6 | Medium |
| Deck Audio Playback | Medium (9/12) | 3 | Low |

---

## Recommended Flows

### 1. Local Folder Scan (Priority: High)

**Why document this?**
This is the primary way music enters the library and involves the most components end-to-end: native dialog, filesystem traversal, audio metadata extraction, deterministic pseudo-analysis (the "magic" BPM/key/energy generation), deduplication, and smart crate rebuilding. Understanding the pseudo-analysis pipeline is critical for anyone extending the app with real DSP.

**Trigger**: User clicks "Scan Folder" button in UI

**Key components**:
- `App.tsx` (handleChooseFolder) - initiates dialog and IPC call
- `tauri-plugin-dialog` - native folder picker
- `scan_music_folder` command - Tauri IPC entry point
- `scan_local_folder` - walkdir traversal, audio extension filtering
- `extract_local_track` - lofty metadata reading
- `analyze_track` - deterministic BPM/key/energy/waveform generation
- `merge_tracks` - deduplication and Hybrid source promotion
- `refresh_snapshot` - smart crate rebuild, stats recalculation

**Scoring**:
| Criterion | Score | Rationale |
|-----------|-------|-----------|
| Frequency | 3 | Primary way to populate the library |
| Complexity | 3 | 7+ components across frontend/backend, filesystem I/O, metadata parsing |
| Mystery | 3 | Hash-based pseudo-analysis is opaque -- how does a filename become a BPM? |
| Debug value | 3 | Failures here = empty library, wrong metadata, missing tracks |

**Key files to start tracing**:
- `src/App.tsx:134-154` - handleChooseFolder triggers dialog then IPC
- `src-tauri/src/lib.rs:347-402` - scan_music_folder command handler
- `src-tauri/src/lib.rs:720-750` - scan_local_folder walks directory
- `src-tauri/src/lib.rs:752-810` - extract_local_track reads tags
- `src-tauri/src/lib.rs:821-906` - analyze_track pseudo-analysis
- `src-tauri/src/lib.rs:938-1008` - merge_tracks deduplication

**Prompt to use**:
```
Create code flow documentation for PulseGrid DJ covering:
Local Folder Scan - from folder picker dialog through walkdir traversal, lofty metadata extraction, hash-based pseudo-analysis (BPM/key/energy/waveform), merge_tracks deduplication, to refresh_snapshot with smart crate rebuilding.

Reference the architecture overview at .pf-agent-system-mapper/output/01-architecture-overview.md
Start tracing from src/App.tsx:134 (handleChooseFolder)
```

---

### 2. Mix Assistant Scoring (Priority: High)

**Why document this?**
The mix assistant is the app's differentiating feature -- the "AI copilot" that scores track compatibility using weighted BPM distance, Camelot wheel key compatibility, and energy targeting. Understanding this algorithm is essential for tuning suggestions and eventually connecting real analysis data.

**Trigger**: Automatically fires when a track is loaded to either deck, or when the energy target slider changes

**Key components**:
- `App.tsx` (useEffect) - auto-triggers on deck/energy state change
- `build_mix_assistant` command - Tauri IPC entry point
- `generate_mix_assistant` - candidate scoring and ranking
- `compatibility_score` - weighted BPM (0.36) + key (0.34) + energy (0.30) calculation
- `key_compatible` - Camelot wheel logic (same number diff letter, adjacent numbers same letter)

**Scoring**:
| Criterion | Score | Rationale |
|-----------|-------|-----------|
| Frequency | 3 | Fires automatically on every deck change |
| Complexity | 2 | Concentrated in ~100 lines but multi-factor algorithm |
| Mystery | 3 | Scoring weights and Camelot wheel logic are non-obvious |
| Debug value | 2 | "Why did it suggest this track?" requires understanding weights |

**Key files to start tracing**:
- `src/App.tsx:104-132` - useEffect that triggers assistant rebuild
- `src-tauri/src/lib.rs:404-423` - build_mix_assistant command
- `src-tauri/src/lib.rs:1023-1130` - generate_mix_assistant scoring
- `src-tauri/src/lib.rs:1132-1142` - compatibility_score weights
- `src-tauri/src/lib.rs:1144-1166` - key_compatible Camelot wheel

**Prompt to use**:
```
Create code flow documentation for PulseGrid DJ covering:
Mix Assistant Scoring - from deck load/energy change trigger through candidate filtering, compatibility_score weighted calculation (BPM 0.36 + key 0.34 + energy 0.30), Camelot wheel key_compatible logic, top-5 selection, to insight generation.

Reference the architecture overview at .pf-agent-system-mapper/output/01-architecture-overview.md
Start tracing from src/App.tsx:104 (useEffect assistant trigger)
```

---

### 3. Spotify Import (Priority: High)

**Why document this?**
This flow demonstrates the external API integration pattern -- paginated Spotify Web API calls, track format conversion, the graceful demo fallback when no token exists, and the critical merge_tracks deduplication that promotes Local/Spotify tracks to Hybrid. Important for extending to other music services.

**Trigger**: User clicks "Import Spotify Library" button

**Key components**:
- `App.tsx` (handleImportSpotify) - initiates IPC call with optional token
- `import_spotify_library` command - Tauri IPC entry point, decides real vs demo import
- `fetch_spotify_library` - orchestrates paginated API calls (saved tracks + playlists)
- `fetch_saved_tracks` / `fetch_playlists` / `fetch_playlist_tracks` - paginated GET endpoints
- `convert_spotify_track` - maps Spotify schema to internal Track with pseudo-analysis
- `merge_tracks` - deduplication, Hybrid promotion
- `demo_spotify_import` - fallback with hardcoded demo data

**Scoring**:
| Criterion | Score | Rationale |
|-----------|-------|-----------|
| Frequency | 2 | Typically done once or occasionally re-imported |
| Complexity | 3 | Pagination, API error handling, format conversion, demo fallback |
| Mystery | 2 | API integration pattern is familiar, but demo fallback branching is subtle |
| Debug value | 3 | Token issues, API failures, missing tracks are common debug scenarios |

**Key files to start tracing**:
- `src/App.tsx:172-182` - handleImportSpotify
- `src-tauri/src/lib.rs:263-345` - import_spotify_library command
- `src-tauri/src/lib.rs:1175-1228` - fetch_spotify_library orchestrator
- `src-tauri/src/lib.rs:1230-1270` - fetch_saved_tracks pagination
- `src-tauri/src/lib.rs:1394-1460` - demo_spotify_import fallback

**Prompt to use**:
```
Create code flow documentation for PulseGrid DJ covering:
Spotify Import - from import button click through token validation, paginated Spotify Web API calls (saved tracks + playlists), convert_spotify_track format mapping, merge_tracks deduplication with Hybrid promotion, to demo_spotify_import graceful fallback.

Reference the architecture overview at .pf-agent-system-mapper/output/01-architecture-overview.md
Start tracing from src/App.tsx:172 (handleImportSpotify)
```

---

### 4. Deck Audio Playback (Priority: Medium)

**Why document this?**
The useDeck hook manages dual-mode playback: real audio via HTMLAudioElement + Tauri asset protocol for local files, and a visual-mode simulation timer for metadata-only (Spotify) tracks. The branching between these modes and the beat-sync logic are non-obvious.

**Trigger**: User loads a track to a deck and presses play

**Key components**:
- `DeckPanel.tsx` - deck UI with transport controls
- `useDeck` hook - state machine: load, play, pause, stop, seek, eject, tempo, volume
- Tauri `convertFileSrc` - resolves local paths to asset:// URLs
- HTMLAudioElement - browser audio API for actual playback

**Scoring**:
| Criterion | Score | Rationale |
|-----------|-------|-----------|
| Frequency | 3 | Core user interaction -- playing music |
| Complexity | 2 | Single hook but dual-mode branching (real audio vs visual sim) |
| Mystery | 2 | Visual-mode timer fallback is unexpected behavior |
| Debug value | 2 | Playback failures, sync issues |

**Key files to start tracing**:
- `src/components/DeckPanel.tsx:56-212` - deck UI and transport
- `src/hooks/useDeck.ts:30-281` - deck state machine and audio management
- `src/App.tsx:184-220` - track loading into decks

**Prompt to use**:
```
Create code flow documentation for PulseGrid DJ covering:
Deck Audio Playback - from track load through convertFileSrc asset resolution, HTMLAudioElement lifecycle, visual-mode timer simulation for metadata-only tracks, tempo/pitch control, beat sync via BPM ratio, to cue point navigation.

Reference the architecture overview at .pf-agent-system-mapper/output/01-architecture-overview.md
Start tracing from src/hooks/useDeck.ts:30 (useDeck hook)
```

---

## Skip These (Low Value)

| Flow | Why Skip |
|------|----------|
| `bootstrap_app` | Simple seeding: checks if empty, calls `seed_snapshot()` with hardcoded demo data. Single function, no branching, no external I/O. |
| `save_spotify_config` | Trivial state mutation: stores 3 fields in `PersistentState`, returns snapshot. ~40 lines, no complexity. |
| Smart Crate Rebuilding | Already covered as a sub-step of Local Folder Scan and Spotify Import flows (via `refresh_snapshot`). Documenting standalone would duplicate. |

---

## Notes

- **Recommended order**: Local Folder Scan first (covers the most components and reveals the pseudo-analysis pipeline), then Mix Assistant (the differentiating feature), then Spotify Import (extends the merge_tracks pattern already seen in scan).
- **Shared components**: `merge_tracks` and `refresh_snapshot` appear in both Local Folder Scan and Spotify Import -- documenting the scan flow first means the import flow can reference it.
- **Single-file caveat**: All backend flows live in `lib.rs` (1527 lines). Line numbers are the primary navigation aid since there are no module boundaries.
