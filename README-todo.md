# PulseGrid DJ - Task Tracker

## Completed

- [x] Project scaffold (Tauri 2 + React 19 + TypeScript + Vite)
- [x] Rust backend with 5 IPC commands in single lib.rs
- [x] Local folder scanning with walkdir + lofty metadata extraction
- [x] Deterministic pseudo-analysis (BPM, key, energy, waveform, cue points)
- [x] Spotify metadata import (Web API + demo fallback)
- [x] Track deduplication and hybrid merging
- [x] Smart crate auto-generation (Warmup, Peak, Open Format, After Hours)
- [x] Dual deck UI with waveform, transport controls, cue bank
- [x] Mix assistant with BPM/key/energy compatibility scoring
- [x] Camelot wheel harmonic mixing logic
- [x] Library table with search, filter, sort
- [x] Dark theme CSS with glass/blur aesthetics
- [x] Agent System Mapper documentation pipeline installed
- [x] Architecture overview documented (01-architecture-overview.md)
- [x] Code flow recommendations generated
- [x] Local Folder Scan flow fully documented

## In Progress

- [ ] Document remaining code flows (Mix Assistant, Spotify Import, Deck Playback)

## Backlog - Known Issues from Code Flow Analysis

- [ ] Extract year from local audio file tags (currently hardcoded None)
- [ ] Extract artwork from local audio files (currently hardcoded None)
- [ ] Add logging for silent tag read failures in extract_local_track
- [ ] Make scan_music_folder async to avoid blocking command thread on large folders
- [ ] Handle stale tracks on re-scan (remove files no longer on disk)
- [ ] Add scan progress reporting to frontend
- [ ] Implement real Spotify OAuth flow (currently manual token paste)

## Backlog - Architecture Improvements

- [ ] Decompose lib.rs (1527 lines) into modules (commands, models, spotify, analysis, utils)
- [ ] Add persistent storage (SQLite or file-based)
- [ ] Add React Error Boundaries
- [ ] Tighten CSP (remove unsafe-inline)
- [ ] Extract demo/seed data from production code
- [ ] Add unit tests (Rust + TypeScript)
- [ ] Set up CI/CD pipeline
- [ ] Initialize git repository and push to GitHub
