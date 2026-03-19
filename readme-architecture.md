# PulseGrid DJ - Architecture

## System Classification

Desktop application built with Tauri 2 (Rust backend) + React 19 SPA (TypeScript frontend).

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Desktop Framework | Tauri | 2.x |
| Backend | Rust | Edition 2024 |
| Frontend | React + TypeScript | 19.x / 5.7+ |
| Build | Vite | 7.x |
| Audio Metadata | lofty | 0.23 |
| HTTP Client | reqwest | 0.12 (rustls-tls) |
| Directory Traversal | walkdir | 2 |
| Styling | Plain CSS (dark theme) | - |

## Architecture Pattern

Monolithic single-file Rust backend (`src-tauri/src/lib.rs`, ~1527 lines) communicating with a component-based React frontend via Tauri IPC.

### State Management

- **Backend**: Single `Mutex<PersistentState>` wrapping `AppSnapshot` + Spotify token. All state is in-memory, no persistence.
- **Frontend**: React `useState` in App.tsx, passed as props. No external state library.
- **Sync pattern**: Every backend command returns a complete `AppSnapshot`. No incremental updates.

## IPC Commands (5 total)

| Command | Type | Purpose |
|---------|------|---------|
| `bootstrap_app` | sync | Initialize app, seed demo data |
| `save_spotify_config` | sync | Store Spotify credentials |
| `import_spotify_library` | async | Fetch Spotify playlists/liked songs or demo fallback |
| `scan_music_folder` | sync | Walk directory, extract metadata, merge into library |
| `build_mix_assistant` | sync | Score tracks by BPM/key/energy compatibility |

## Component Map

### Backend (src-tauri/src/)

```
main.rs          - Entry: calls lib::run()
lib.rs           - ALL backend logic:
  |- Data models   (Track, Crate, AppSnapshot, etc.)
  |- IPC commands  (5 #[tauri::command] functions)
  |- Folder scan   (walkdir + lofty + pseudo-analysis)
  |- Spotify API   (reqwest + pagination + demo fallback)
  |- Mix assistant  (compatibility scoring + Camelot wheel)
  |- Smart crates   (energy/BPM/genre-based auto-crates)
  |- Utilities      (stable_hash, normalize_text, split_tags)
```

### Frontend (src/)

```
App.tsx              - Root orchestrator, all state management
components/
  DeckPanel.tsx      - Deck UI (waveform, transport, cues)
  Sidebar.tsx        - Stats, crate browser, Spotify status
  LibraryTable.tsx   - Track table with sort/filter/load
  AssistantPanel.tsx - Mix suggestions and set advice
hooks/
  useDeck.ts         - Deck state machine + audio element
lib/
  tauri.ts           - Typed IPC invoke wrappers
  types.ts           - TypeScript interfaces mirroring Rust structs
```

## Key Data Flow

```
User Action -> React Component -> tauri.ts invoke() -> Tauri IPC
    -> Rust command -> Mutex lock -> mutate AppSnapshot -> refresh_snapshot()
    -> clone snapshot -> IPC response -> applySnapshot() -> React re-render
```

## Key Design Decisions

1. **Snapshot-based sync**: Simple but doesn't scale well with large libraries
2. **Pseudo-analysis**: BPM/key/energy from metadata hashes, not DSP. MVP shortcut.
3. **Hybrid track merging**: Same track from local + Spotify becomes Hybrid source
4. **Graceful Spotify fallback**: Demo data when no token, keeps full UI functional
5. **Camelot wheel compatibility**: DJ-standard harmonic mixing for suggestions

## Documentation

Detailed documentation generated via agent-system-mapper:

- `pf-docs/CODE-FLOW-RECOMMENDATIONS.md` - Prioritized flow candidates
- `pf-docs/02-code-flow-local-folder-scan.md` - Full Local Folder Scan trace
- `.pf-agent-system-mapper/output/01-architecture-overview.md` - Verified architecture overview (52 verified claims)
