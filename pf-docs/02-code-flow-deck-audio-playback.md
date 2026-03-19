# Deck Audio Playback Code Flow

## Metadata
| Field | Value |
|-------|-------|
| Repository | `codex-dac-rust` |
| Commit | `d5a3570` |
| Documented | `2026-03-18` |
| Trigger | User loads a track to a deck and presses Play |
| End State | Audio plays through HTMLAudioElement (local files) or time progresses via interval simulation (metadata-only tracks) |

## Verification Summary
- [VERIFIED]: 26
- [INFERRED]: 1
- [NOT_FOUND]: 0
- [ASSUMED]: 1

---

## Flow Diagram

```
[User clicks "Load A" or "Load B" in LibraryTable / AssistantPanel]
        │
        ▼
  App.tsx::loadDeck(track, deckId)
        │
        ├──→ useDeck::loadTrack(track)  [state reset]
        │
        ▼
  useEffect[track.id, track.path]  [audio source setup]
        │
        ├── track.path present?
        │     ├── YES ──→ convertFileSrc(path) → asset:// URL
        │     │              │
        │     │              └──→ audio.src = URL; audio.load()
        │     │
        │     └── NO ──→ audio.pause(); audio.removeAttribute("src")
        │                    │
        │                    └──→ Visual mode (no audio element source)
        │
        ▼
  [User clicks "Play"]
        │
        ├──→ useDeck::togglePlay()
        │        │
        │        ├── track.path present?
        │        │     ├── YES ──→ audio.play()  [HTMLAudioElement]
        │        │     │              │
        │        │     │              ▼
        │        │     │           timeupdate events ──→ syncFromAudio()
        │        │     │              │
        │        │     │              ▼
        │        │     │           setState(current_time, playing)
        │        │     │
        │        │     └── NO ──→ setState(playing: true)
        │        │                    │
        │        │                    ▼
        │        │                 useEffect[playing, rate, track]
        │        │                    │
        │        │                    └──→ setInterval(120ms)
        │        │                            │
        │        │                            ▼
        │        │                         current_time += 0.12 * rate
        │        │
        │        ▼
        │    DeckPanel re-renders
        │        │
        │        ├── Waveform bars: active/inactive based on current_time
        │        ├── Progress bar: width from current_time / duration
        │        ├── Clock: formatClock(current_time) / formatClock(duration)
        │        └── Transport: "Play" ↔ "Pause" label
        │
        ▼
  [Track ends]
        │
        ├── Real audio ──→ "ended" event ──→ handleEnded()
        │                                        │
        │                                        └──→ playing: false, current_time: 0
        │
        └── Visual mode ──→ nextTime >= duration ──→ playing: false
```

---

## Detailed Flow

### Step 1: Track Loading — App.tsx

[VERIFIED: src/App.tsx:184-193]
```tsx
function loadDeck(track: Track, deckId: "A" | "B") {
    if (deckId === "A") {
        deckA.loadTrack(track);
    } else {
        deckB.loadTrack(track);
    }
    setSelectedTrackId(track.id);
    setStatusLine(`${track.title} loaded to Deck ${deckId}.`);
}
```

Called from two sources:

1. **LibraryTable** — "A" and "B" load buttons per track row
   [VERIFIED: src/App.tsx:397-398]
   ```tsx
   onLoadDeckA={(track) => loadDeck(track, "A")}
   onLoadDeckB={(track) => loadDeck(track, "B")}
   ```

2. **AssistantPanel** — loading a suggestion always targets Deck B
   [VERIFIED: src/App.tsx:408]
   ```tsx
   onLoadSuggestion={(track) => loadDeck(track, "B")}
   ```

**Calls:** `useDeck::loadTrack(track)`

---

### Step 2: State Reset — useDeck::loadTrack

[VERIFIED: src/hooks/useDeck.ts:181-191]
```tsx
function loadTrack(track: Track) {
    setState((current) => ({
        ...current,
        track,
        playing: false,
        current_time: 0,
        display_duration: track.duration_seconds,
        rate: 1,
        pitch_percent: 0,
    }));
}
```

Resets all playback state: stops playing, rewinds to 0, resets tempo to 1x. Preserves `volume` and `deck_id`. The `display_duration` is set from the track's metadata duration.

**Data in:** `Track` object
**Data out:** Updated `DeckState` with new track, all transport reset

---

### Step 3: Audio Source Setup — useEffect on track change

[VERIFIED: src/hooks/useDeck.ts:97-129]
```tsx
useEffect(() => {
    const audio = audioRef.current;

    if (!audio) {
        return;
    }

    if (!state.track || !state.track.path) {
        audio.pause();
        audio.removeAttribute("src");
        audio.load();

        setState((current) => ({
            ...current,
            playing: false,
            current_time: 0,
            display_duration: current.track?.duration_seconds ?? current.display_duration,
        }));

        return;
    }

    audio.pause();
    audio.src = convertFileSrc(state.track.path);
    audio.load();

    setState((current) => ({
        ...current,
        playing: false,
        current_time: 0,
        display_duration: state.track?.duration_seconds || current.display_duration,
    }));
}, [state.track?.id, state.track?.path]);
```

Triggered when `track.id` or `track.path` changes. Two branches:

#### Branch A: Track has local path (real audio)

1. Pauses any currently playing audio
2. Calls `convertFileSrc(state.track.path)` to convert the filesystem path to a Tauri `asset://` URL
3. Sets `audio.src` to the asset URL
4. Calls `audio.load()` to begin loading the audio resource
5. Resets playback state

[VERIFIED: src/hooks/useDeck.ts:1]
```tsx
import { convertFileSrc } from "@tauri-apps/api/core";
```

[ASSUMED: `convertFileSrc` maps a local path like `/Users/dj/Music/track.mp3` to `asset://localhost/Users/dj/Music/track.mp3` via Tauri's asset protocol, configured with scope `["**"]` in tauri.conf.json]

#### Branch B: No path (metadata-only / Spotify track)

1. Pauses audio
2. Removes `src` attribute entirely
3. Calls `audio.load()` to reset the audio element
4. Sets `display_duration` from track metadata (so waveform and progress bar still work)

This puts the deck in **visual mode** — no audio source, but UI remains fully functional.

---

### Step 4: Audio Element — DeckPanel

[VERIFIED: src/components/DeckPanel.tsx:78]
```tsx
<audio ref={audioRef} preload="metadata" />
```

A hidden `<audio>` element with `preload="metadata"` — the browser loads enough of the file to read duration and codec info. The `audioRef` is passed from `useDeck` through App.tsx to DeckPanel.

[VERIFIED: src/App.tsx:359]
```tsx
audioRef={deckA.audioRef}
```

---

### Step 5: Audio Event Listeners

[VERIFIED: src/hooks/useDeck.ts:59-84]
```tsx
useEffect(() => {
    const audio = audioRef.current;
    if (!audio) { return; }

    const onTimeUpdate = () => syncFromAudio();
    const onLoadedMetadata = () => syncFromAudio();
    const onPause = () => syncFromAudio();
    const onPlay = () => syncFromAudio();

    audio.addEventListener("timeupdate", onTimeUpdate);
    audio.addEventListener("loadedmetadata", onLoadedMetadata);
    audio.addEventListener("ended", handleEnded);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("play", onPlay);

    return () => {
        audio.removeEventListener("timeupdate", onTimeUpdate);
        audio.removeEventListener("loadedmetadata", onLoadedMetadata);
        audio.removeEventListener("ended", handleEnded);
        audio.removeEventListener("pause", onPause);
        audio.removeEventListener("play", onPlay);
    };
}, [handleEnded, syncFromAudio]);
```

Five HTMLAudioElement events are bound:

| Event | Handler | Purpose |
|-------|---------|---------|
| `timeupdate` | `syncFromAudio()` | Update `current_time` during playback (~4Hz) |
| `loadedmetadata` | `syncFromAudio()` | Capture actual audio duration after load |
| `pause` | `syncFromAudio()` | Sync playing state on pause |
| `play` | `syncFromAudio()` | Sync playing state on play |
| `ended` | `handleEnded()` | Reset to beginning when track finishes |

---

### Step 6: State Sync from Audio — syncFromAudio

[VERIFIED: src/hooks/useDeck.ts:34-49]
```tsx
const syncFromAudio = useEffectEvent(() => {
    const audio = audioRef.current;

    if (!audio) {
        return;
    }

    setState((current) => ({
        ...current,
        current_time: audio.currentTime,
        display_duration: Number.isFinite(audio.duration) && audio.duration > 0
            ? audio.duration
            : current.display_duration,
        playing: !audio.paused,
    }));
});
```

Reads three values from the HTMLAudioElement and syncs them to React state:
- `current_time` from `audio.currentTime`
- `display_duration` from `audio.duration` (only if finite and positive — protects against `NaN`/`Infinity` during loading)
- `playing` from `!audio.paused`

Uses `useEffectEvent` (React 19) to avoid stale closure issues — the function always reads the latest `audioRef` without needing to be in the dependency array.

---

### Step 7: Track End Handling — handleEnded

[VERIFIED: src/hooks/useDeck.ts:51-57]
```tsx
const handleEnded = useEffectEvent(() => {
    setState((current) => ({
        ...current,
        playing: false,
        current_time: 0,
    }));
});
```

When the audio element fires the `ended` event, playback stops and the position resets to 0. No auto-advance to next track — the DJ must manually load the next track.

---

### Step 8: Play/Pause — togglePlay

[VERIFIED: src/hooks/useDeck.ts:154-179]
```tsx
async function togglePlay() {
    if (!state.track) {
        return;
    }

    const audio = audioRef.current;

    if (state.playing) {
        audio?.pause();
        setState((current) => ({ ...current, playing: false }));
        return;
    }

    if (state.track.path && audio) {
        audio.currentTime = state.current_time;
        audio.playbackRate = state.rate;

        try {
            await audio.play();
        } catch (error) {
            console.warn(`Unable to play Deck ${deckId}`, error);
        }
    }

    setState((current) => ({ ...current, playing: true }));
}
```

Toggle behavior:

| Current State | Action |
|---------------|--------|
| No track loaded | Return immediately |
| Playing | `audio.pause()` + set playing: false |
| Paused, has path | Sync `currentTime` and `playbackRate`, `audio.play()` |
| Paused, no path | Just set `playing: true` (triggers visual mode interval) |

The `audio.play()` call is `async` and wrapped in try/catch — browsers may reject play requests due to autoplay policies. The function always sets `playing: true` at the end (even for visual mode), which triggers the interval-based simulation in Step 9.

---

### Step 9: Visual Mode — Interval-Based Time Simulation

[VERIFIED: src/hooks/useDeck.ts:131-152]
```tsx
useEffect(() => {
    if (!state.playing || !state.track || state.track.path) {
        return;
    }

    const interval = window.setInterval(() => {
        setState((current) => {
            const nextTime = Math.min(
                current.current_time + 0.12 * current.rate,
                current.display_duration || current.track?.duration_seconds || 0,
            );

            return {
                ...current,
                current_time: nextTime,
                playing: nextTime < (current.display_duration || current.track?.duration_seconds || 0),
            };
        });
    }, 120);

    return () => window.clearInterval(interval);
}, [state.playing, state.rate, state.track]);
```

**Guard conditions** — this effect only activates when ALL three are true:
1. `state.playing` is true
2. `state.track` is not null
3. `state.track.path` is **falsy** (no local file — visual mode only)

**Timer mechanics:**
- Fires every **120ms**
- Advances `current_time` by `0.12 * rate` seconds per tick
- At default rate (1.0x): `0.12s / 0.12s = 1.0` — simulates real-time progression
- At 1.22x: `0.12 * 1.22 = 0.1464` seconds per tick — faster progression
- Clamps to `display_duration` — stops at end of track
- Auto-stops by setting `playing: false` when `nextTime >= duration`

**Cleanup:** Clears interval on dependency change or unmount.

[INFERRED: The 120ms interval produces ~8.3 FPS state updates, which is sufficient for smooth waveform bar activation but not for frame-accurate audio sync]

---

### Step 10: Tempo Control — setTempo

[VERIFIED: src/hooks/useDeck.ts:249-262]
```tsx
function setTempo(rate: number) {
    const clamped = Math.max(0.78, Math.min(rate, 1.22));
    const audio = audioRef.current;

    if (audio) {
        audio.playbackRate = clamped;
    }

    setState((current) => ({
        ...current,
        rate: clamped,
        pitch_percent: (clamped - 1) * 100,
    }));
}
```

Tempo range: **0.78x to 1.22x** (±22%). Applies to both real audio (`audio.playbackRate`) and visual mode (via `rate` multiplier in the interval). Also updates `pitch_percent` for UI display.

[VERIFIED: src/components/DeckPanel.tsx:171-179]
```tsx
<input
    max={1.22}
    min={0.78}
    onChange={(event) => onSetTempo(Number(event.target.value))}
    step={0.005}
    type="range"
    value={deck.rate}
/>
```

Slider step is 0.005 (0.5% increments).

---

### Step 11: Volume Control — setVolume

[VERIFIED: src/hooks/useDeck.ts:240-247]
```tsx
function setVolume(volume: number) {
    const clamped = Math.max(0, Math.min(volume, 1));
    setState((current) => ({
        ...current,
        volume: clamped,
    }));
}
```

Volume range: 0.0 to 1.0. Applied to the audio element via a separate useEffect:

[VERIFIED: src/hooks/useDeck.ts:86-95]
```tsx
useEffect(() => {
    const audio = audioRef.current;
    if (!audio) { return; }
    audio.volume = state.volume;
    audio.playbackRate = state.rate;
}, [state.volume, state.rate]);
```

Default volumes differ per deck:
[VERIFIED: src/hooks/useDeck.ts:24]
```tsx
volume: deckId === "A" ? 0.92 : 0.88,
```

---

### Step 12: Seek and Cue Navigation

[VERIFIED: src/hooks/useDeck.ts:222-238]
```tsx
function seek(timeInSeconds: number) {
    const clamped = Math.max(0, Math.min(timeInSeconds, state.display_duration || state.track?.duration_seconds || 0));
    const audio = audioRef.current;

    if (audio && state.track?.path) {
        audio.currentTime = clamped;
    }

    setState((current) => ({
        ...current,
        current_time: clamped,
    }));
}

function jumpToCue(cue: CuePoint) {
    seek(cue.time_seconds);
}
```

`seek()` clamps to [0, duration], updates both the audio element (if real audio) and React state. Works in both real and visual modes.

`jumpToCue()` is a thin wrapper — delegates to `seek(cue.time_seconds)`.

Waveform bars are also clickable seek targets:
[VERIFIED: src/components/DeckPanel.tsx:119-127]
```tsx
const barTime = (index / waveform.length) * duration;
// ...
onClick={() => onSeek(barTime)}
```

Each of the 72 waveform bars maps to a proportional time position. Clicking a bar seeks to that position.

---

### Step 13: Nudge

[VERIFIED: src/hooks/useDeck.ts:264-266]
```tsx
function nudge(seconds: number) {
    seek(state.current_time + seconds);
}
```

Offset seek by a fixed number of seconds. UI provides ±8 second nudge buttons:

[VERIFIED: src/components/DeckPanel.tsx:148-158]
```tsx
<button className="transport ghost" onClick={() => onNudge(-8)} type="button">-8</button>
// ...
<button className="transport ghost" onClick={() => onNudge(8)} type="button">+8</button>
```

---

### Step 14: Beat Sync

[VERIFIED: src/App.tsx:195-206]
```tsx
function handleSync(deckId: "A" | "B") {
    const targetDeck = deckId === "A" ? deckA : deckB;
    const referenceDeck = deckId === "A" ? deckB : deckA;

    if (!targetDeck.state.track || !referenceDeck.state.track) {
        return;
    }

    const nextRate = referenceDeck.state.track.bpm / targetDeck.state.track.bpm;
    targetDeck.setTempo(nextRate);
    setStatusLine(`Deck ${deckId} synced to Deck ${referenceDeck.state.deck_id}.`);
}
```

Calculates the tempo ratio needed to match the target deck's BPM to the reference deck's BPM. For example: reference at 128 BPM, target at 124 BPM → `128/124 = 1.032x`.

Both decks must have tracks loaded (`canSync` prop controls button disabled state).

[VERIFIED: src/App.tsx:360]
```tsx
canSync={Boolean(deckA.state.track && deckB.state.track)}
```

Note: This is **tempo sync only** — not phase/beat alignment. The playback positions are not adjusted.

---

### Step 15: Eject

[VERIFIED: src/hooks/useDeck.ts:193-205]
```tsx
function ejectTrack() {
    const audio = audioRef.current;
    audio?.pause();

    setState((current) => ({
        ...current,
        track: null,
        current_time: 0,
        display_duration: 0,
        playing: false,
    }));
}
```

Pauses audio and clears all deck state. Sets `track: null` which triggers the useEffect at Step 3 (Branch B), clearing the audio source.

---

### Step 16: Stop

[VERIFIED: src/hooks/useDeck.ts:207-220]
```tsx
function stop() {
    const audio = audioRef.current;

    if (audio) {
        audio.pause();
        audio.currentTime = 0;
    }

    setState((current) => ({
        ...current,
        current_time: 0,
        playing: false,
    }));
}
```

Pauses audio and resets position to 0, but keeps the track loaded (unlike eject). The track can be replayed from the beginning.

---

### Step 17: Waveform Visualization — DeckPanel

[VERIFIED: src/components/DeckPanel.tsx:72-74]
```tsx
const duration = deck.display_duration || deck.track?.duration_seconds || 0;
const progress = duration > 0 ? deck.current_time / duration : 0;
const waveform = deck.track?.waveform ?? Array.from({ length: 72 }, (_, index) => 0.12 + ((index % 5) + 1) * 0.08);
```

If no track is loaded, a fallback waveform is generated with a repeating 5-step pattern. Otherwise uses the 72-float array from pseudo-analysis.

[VERIFIED: src/components/DeckPanel.tsx:118-135]
```tsx
{waveform.map((value, index) => {
    const barTime = (index / waveform.length) * duration;
    const active = barTime <= deck.current_time;

    return (
        <button
            key={`${deck.deck_id}-${index}`}
            aria-label={`Seek to ${formatClock(barTime)}`}
            className={`wave-bar ${active ? "is-active" : ""}`}
            onClick={() => onSeek(barTime)}
            style={{
                height: `${Math.max(10, value * 100)}%`,
                ["--accent" as string]: accent,
            }}
            type="button"
        />
    );
})}
```

72 bars rendered as `<button>` elements:
- Height: proportional to waveform value (min 10%)
- Active state: `is-active` class when `barTime <= current_time` (lights up as playback progresses)
- Clickable: each bar is a seek target
- Accessible: `aria-label` with formatted time

**Visual update cadence:**
- Real audio: ~4Hz from `timeupdate` events → ~every 18 bars
- Visual mode: ~8.3Hz from 120ms interval → smoother bar-by-bar activation

---

## External Calls

| Call | Location | Details |
|------|----------|---------|
| `convertFileSrc()` | `src/hooks/useDeck.ts:120` | Converts local path to `asset://` URL. No network I/O. |
| Tauri asset protocol | Browser level | Serves local file bytes to `<audio>` element via internal protocol. No network I/O. |

No HTTP requests, no backend IPC commands, no database queries in the playback flow itself.

---

## Events Fired

No custom events. The flow is driven by HTMLAudioElement DOM events (`timeupdate`, `loadedmetadata`, `ended`, `pause`, `play`) and React state/effect reactivity.

---

## Data Shape at Key Boundaries

### Track Loading Input
```typescript
Track {
    id: "local-14928371049283",
    source: "local",
    path: "/Users/dj/Music/House/track.mp3",  // or null for Spotify
    title: "Night Traction",
    duration_seconds: 417.0,
    bpm: 126.0,
    musical_key: "8A",
    energy: 0.78,
    waveform: [0.45, 0.62, ...],  // 72 floats
    cue_points: [
        { label: "Intro", time_seconds: 0, color: "#ffcd6f" },
        { label: "Blend", time_seconds: 75, color: "#29f0b4" },
        { label: "Drop", time_seconds: 142, color: "#ff7a18" },
        { label: "Break", time_seconds: 238, color: "#88a8ff" },
        { label: "Outro", time_seconds: 342, color: "#ff4858" },
    ],
    // ...remaining fields
}
```

### DeckState During Playback
```typescript
DeckState {
    deck_id: "A",
    track: Track,           // loaded track object
    playing: true,
    current_time: 142.5,    // seconds into playback
    display_duration: 417.0,// from audio element or track metadata
    volume: 0.92,
    rate: 1.032,            // adjusted tempo (post beat-sync)
    pitch_percent: 3.2,     // (1.032 - 1) * 100
}
```

---

## Two-Mode Architecture Summary

| Aspect | Real Audio Mode | Visual Mode |
|--------|----------------|-------------|
| **Condition** | `track.path` is present | `track.path` is null |
| **Audio source** | `asset://` URL via `convertFileSrc` | No `<audio>` source |
| **Time progression** | HTMLAudioElement `timeupdate` events | `setInterval(120ms)` simulation |
| **Tempo effect** | `audio.playbackRate` changes actual speed | `rate` multiplier in interval math |
| **Volume** | `audio.volume` controls output | No effect (no audio output) |
| **Seek** | `audio.currentTime = clamped` | State update only |
| **Status chip** | "Audio armed" | "Visual mode" |
| **Track end** | `ended` DOM event | `nextTime >= duration` check |

[VERIFIED: src/App.tsx:371]
```tsx
statusText={deckA.state.track?.path ? "Audio armed" : "Visual mode"}
```

---

## Known Issues Found

### 1. No crossfader
[NOT_FOUND: searched "crossfade", "crossfader", "xfade", "mix" in src/components/ and src/hooks/]
There is no crossfader control between the two decks. Volume control is per-deck only. DJs typically need a crossfader for smooth transitions.

### 2. Beat sync is tempo-only, not phase-aligned
[VERIFIED: src/App.tsx:203]
```tsx
const nextRate = referenceDeck.state.track.bpm / targetDeck.state.track.bpm;
targetDeck.setTempo(nextRate);
```
BPM matching via tempo ratio doesn't align beat phases. Two tracks at the same BPM can still be off-beat. Real DJ software aligns beat grids.

### 3. Tempo clamping can prevent beat sync
[VERIFIED: src/hooks/useDeck.ts:250]
```tsx
const clamped = Math.max(0.78, Math.min(rate, 1.22));
```
If the BPM ratio exceeds ±22%, the tempo is clamped and sync won't actually match BPMs. For example, syncing 90 BPM to 128 BPM needs a 1.42x ratio, but it's clamped to 1.22x.

### 4. Visual mode timer drift
[VERIFIED: src/hooks/useDeck.ts:136-149]
The 120ms `setInterval` is not drift-compensated. JavaScript timers are not precise — over a 6-minute track, accumulated drift could desync the visual position from where actual audio would be. No `requestAnimationFrame` or drift correction is used.

### 5. No EQ or effects
[NOT_FOUND: searched "filter", "equalizer", "EQ", "Web Audio", "AudioContext" in src/]
No Web Audio API integration. The `<audio>` element provides no frequency control. Real DJ apps use Web Audio nodes for EQ (low/mid/high), filters, and effects.

### 6. Volume sliders don't affect visual mode
Volume has no audible effect in visual mode (no audio source), but the slider still renders and updates state. This could confuse users who don't realize the track has no audio.

### 7. Default volume asymmetry undocumented
[VERIFIED: src/hooks/useDeck.ts:24]
```tsx
volume: deckId === "A" ? 0.92 : 0.88,
```
Deck A starts at 92% volume, Deck B at 88%. This asymmetry isn't explained in the UI or documented. Likely a subtle design choice to give the "main" deck slightly more presence.

### 8. useEffectEvent is experimental
[VERIFIED: src/hooks/useDeck.ts:2]
```tsx
import { useEffect, useEffectEvent, useRef, useState } from "react";
```
`useEffectEvent` is a React 19 API that was experimental for most of React 18's lifecycle. While stable in React 19, it's a less common pattern that may surprise contributors.
