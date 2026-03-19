# PulseGrid DJ

PulseGrid DJ is a desktop-first MVP DJ application built with Rust, Tauri, React, and TypeScript. It combines local music folder scanning, Spotify metadata import, a unified library, dual decks, waveform-driven transport controls, and an AI mix assistant inspired by Serato and Pioneer-style workflows.

## What is included

- Unified music library with:
  - local folder scanning
  - Spotify playlist and liked-track metadata import
  - deduping and hybrid track merging
- Track metadata display:
  - title
  - artist
  - album
  - duration
  - BPM
  - key
  - energy
  - genre or tags
- DJ workflow UI:
  - deck A and deck B
  - waveform view
  - transport controls
  - cue points
  - beat sync
  - library browser
  - crates and smart crates
  - search, filter, and sort controls
- AI-assisted set guidance:
  - next-track suggestions
  - energy arc targeting
  - harmonic and tempo compatibility scoring
  - transition strategy notes

## Stack

- Tauri 2
- Rust backend for scanning, metadata shaping, Spotify import, and mix intelligence
- React 19 + TypeScript frontend
- Vite for the UI build

## Run locally

```bash
npm install
npm run tauri dev
```

## Spotify import notes

This MVP supports a pragmatic Spotify flow:

- Paste a Spotify user access token into the UI.
- Import playlists and liked tracks through the Web API.
- If no token is present, the app falls back to curated demo Spotify metadata so the interface is still usable.

Recommended Spotify scopes:

- `playlist-read-private`
- `playlist-read-collaborative`
- `user-library-read`

## Important MVP note

Spotify imports are metadata-only. Actual audio playback in the decks works with local tracks that come from a scanned music folder. Demo and Spotify-only tracks still work in visual mode so transport, cue, sync, and assistant flows can be explored before loading local audio.
