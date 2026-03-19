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

## Stem Separation

PulseGrid DJ supports on-demand AI stem separation via [Demucs](https://github.com/facebookresearch/demucs).

### Setup

1. Install [uv](https://docs.astral.sh/uv/) (Python package manager):
   ```bash
   curl -LsSf https://astral.sh/uv/install.sh | sh
   ```

2. That's it — the first stem separation will automatically download Demucs, PyTorch, and the htdemucs model (~300MB). This takes a few minutes on first run.

### Usage

1. Scan a local music folder into the library
2. Click **Stems** on any local track in the library table
3. Choose **2-stem** (vocals + instrumental) or **4-stem** (vocals, drums, bass, other)
4. Wait for separation to complete (30-120 seconds per track depending on hardware)
5. Expand the stem group with the **Stems** toggle — each stem is loadable to any deck

### Output

Stems are saved alongside the original file at:
```
{track_directory}/stems/htdemucs/{track_name}/vocals.wav
{track_directory}/stems/htdemucs/{track_name}/drums.wav
...
```

Re-clicking Stems on a track that's already been separated will use the cached output.

### Hardware notes

- **Apple Silicon**: Demucs uses MPS acceleration automatically
- **NVIDIA GPU**: Demucs uses CUDA if available
- **CPU-only**: Works but slower (2-5 minutes per track)

## Important MVP note

Spotify imports are metadata-only. Actual audio playback in the decks works with local tracks that come from a scanned music folder. Demo and Spotify-only tracks still work in visual mode so transport, cue, sync, and assistant flows can be explored before loading local audio.
