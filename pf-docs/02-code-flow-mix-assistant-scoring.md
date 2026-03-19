# Mix Assistant Scoring Code Flow

## Metadata
| Field | Value |
|-------|-------|
| Repository | `codex-dac-rust` |
| Commit | `e951cfe` |
| Documented | `2026-03-18` |
| Trigger | Track loaded to deck, deck ejected, or energy target slider changed |
| End State | `MixAssistantPayload` with top-5 suggestions, compatibility scores, and set insights applied to React state |

## Verification Summary
- [VERIFIED]: 22
- [INFERRED]: 1
- [NOT_FOUND]: 1
- [ASSUMED]: 0

---

## Flow Diagram

```
[Deck load / eject / energy slider change]
        │
        ▼
  App.tsx useEffect (deps: tracks.length, deckA.id, deckB.id, targetEnergy)
        │
        ├──→ buildMixAssistant(deckA?, deckB?, energy)  [IPC invoke]
        │        │
        │        ▼
        │    lib.rs::build_mix_assistant()
        │        │
        │        ├──→ Mutex lock, read snapshot.tracks
        │        │
        │        └──→ generate_mix_assistant()
        │                 │
        │                 ├──→ Resolve anchor track (deck B > deck A > tracks[0])
        │                 │
        │                 ├──→ For each candidate track:
        │                 │        │
        │                 │        └──→ compatibility_score(anchor, candidate, target_energy)
        │                 │                 │
        │                 │                 ├──→ BPM score: 1 - |Δbpm| / 12
        │                 │                 ├──→ Key score: key_compatible() → 1.0 or 0.42
        │                 │                 │        │
        │                 │                 │        └──→ split_key() → Camelot wheel check
        │                 │                 └──→ Energy score: 1 - |Δenergy| / 0.35
        │                 │
        │                 ├──→ Sort by compatibility descending
        │                 ├──→ Truncate to top 5
        │                 ├──→ Generate reason text per suggestion
        │                 └──→ Build insights (energy arc, deck relationship, cue strategy)
        │
        ▼
  setSnapshot(current => {...current, assistant})  [React state merge]
        │
        ▼
  AssistantPanel re-renders with new suggestions + insights
```

---

## Detailed Flow

### Step 1: Reactive Trigger — useEffect

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

This `useEffect` fires whenever any of four dependencies change:

| Dependency | Changes When |
|------------|-------------|
| `snapshot?.tracks.length` | Folder scan or Spotify import adds tracks |
| `deckA.state.track?.id` | Track loaded to or ejected from Deck A |
| `deckB.state.track?.id` | Track loaded to or ejected from Deck B |
| `targetEnergy` | User adjusts energy target slider in AssistantPanel |

Guards: returns early if `snapshot` is null (app not yet bootstrapped). Uses a `cancelled` flag for stale-closure cleanup on rapid dependency changes.

**Data in:** `deckA.state.track?.id: string | undefined`, `deckB.state.track?.id: string | undefined`, `targetEnergy: number`
**Calls:** `buildMixAssistant()` via IPC

---

### Step 2: Energy Target Source — AssistantPanel

[VERIFIED: src/App.tsx:409]
```tsx
onTargetEnergyChange={setTargetEnergy}
```

[VERIFIED: src/components/AssistantPanel.tsx:11-89]
The energy target slider is in the AssistantPanel component. Changes call `setTargetEnergy` which updates the `targetEnergy` state in App.tsx, triggering the useEffect.

[VERIFIED: src/App.tsx:56]
```tsx
const [targetEnergy, setTargetEnergy] = useState(0.72);
```

Default target energy is `0.72`. On bootstrap, it's overridden from the initial snapshot:

[VERIFIED: src/App.tsx:92]
```tsx
setTargetEnergy(initial.assistant.target_energy);
```

---

### Step 3: Deck Loading Source

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

When a track is loaded via `loadDeck()`, the `useDeck` hook's internal state updates, which changes `deckA.state.track?.id` or `deckB.state.track?.id`, triggering the useEffect at Step 1.

---

### Step 4: Tauri IPC Bridge

[VERIFIED: src/lib/tauri.ts:23-33]
```tsx
export function buildMixAssistant(
    deckATrackId?: string | null,
    deckBTrackId?: string | null,
    targetEnergy?: number | null,
) {
    return invoke<MixAssistantPayload>("build_mix_assistant", {
        deck_a_track_id: deckATrackId ?? null,
        deck_b_track_id: deckBTrackId ?? null,
        target_energy: targetEnergy ?? null,
    });
}
```

Typed wrapper around `invoke()`. Converts `undefined` to `null` for Rust's `Option<T>` deserialization. Returns `Promise<MixAssistantPayload>`.

**Data in:** `{ deck_a_track_id: string | null, deck_b_track_id: string | null, target_energy: number | null }`
**Data out:** `Promise<MixAssistantPayload>`

---

### Step 5: Rust Command Entry — build_mix_assistant

[VERIFIED: src-tauri/src/lib.rs:404-423]
```rust
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
```

Synchronous Tauri command. Acquires a read-only lock on `SharedState` (technically `Mutex` — not `RwLock`), passes the track list and optional parameters to `generate_mix_assistant()`. Does **not** mutate state — this is the only command that returns a payload other than `AppSnapshot`.

**Data in:** `state: SharedState`, `deck_a_track_id: Option<String>`, `deck_b_track_id: Option<String>`, `target_energy: Option<f32>`
**Data out:** `Result<MixAssistantPayload, String>`
**Calls:** `generate_mix_assistant()`

---

### Step 6: Anchor Resolution — generate_mix_assistant

[VERIFIED: src-tauri/src/lib.rs:1023-1042]
```rust
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
```

**Anchor resolution priority:**
1. Deck B track (if loaded) — the "outgoing" deck in a DJ mix
2. Deck A track (if loaded) — fallback
3. First track in the library (`tracks[0]`) — default when no decks loaded

**Target energy resolution:** Use provided value, otherwise default to anchor track's energy.

**Empty library guard:** Returns a placeholder payload with no suggestions and no insights.

**Data in:** `tracks: &[Track]`, optional deck IDs and energy
**Data out (at this point):** `anchor: &Track`, `target_energy: f32`

---

### Step 7: Candidate Scoring Loop

[VERIFIED: src-tauri/src/lib.rs:1044-1068]
```rust
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
```

Iterates all tracks except the anchor. For each candidate:
1. Computes `compatibility_score()` (numeric, 0.0–1.0)
2. Assigns a human-readable `reason` string based on the first matching condition:

| Priority | Condition | Reason |
|----------|-----------|--------|
| 1 | Key compatible AND BPM within 4.0 | "Strong harmonic blend and tight tempo handoff" |
| 2 | Energy within 0.08 | "Energy stays consistent while refreshing the palette" |
| 3 | Candidate energy > anchor energy | "Natural lift for a bigger room response" |
| 4 | (default) | "Recovery option for breathing room between peaks" |

**Data shape per candidate:** `SuggestionTrack { track_id: String, reason: String, compatibility: f32 }`

---

### Step 8: Compatibility Score Calculation

[VERIFIED: src-tauri/src/lib.rs:1132-1142]
```rust
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
```

Three-factor weighted score:

| Factor | Weight | Formula | Range |
|--------|--------|---------|-------|
| **BPM** | 0.36 | `1.0 - (|anchor.bpm - candidate.bpm| / 12.0)` | 0.0–1.0 |
| **Key** | 0.34 | `1.0` if Camelot-compatible, else `0.42` | 0.42 or 1.0 |
| **Energy** | 0.30 | `1.0 - (|candidate.energy - target_energy| / 0.35)` | 0.0–1.0 |

**BPM score**: Perfect at 0 BPM difference, reaches 0.0 at 12+ BPM apart. The `/12.0` divisor means 6 BPM difference = 0.5 score.

**Key score**: Binary — compatible keys get full credit (1.0), incompatible get 0.42 (not zero, so key alone doesn't eliminate a track).

**Energy score**: Perfect when candidate matches target energy exactly, reaches 0.0 at 0.35+ energy difference. Scored against `target_energy`, not anchor energy.

**Final score**: Weighted sum clamped to [0.0, 1.0].

**Example calculation:**
- Anchor: 126 BPM, 8A key, 0.78 energy. Target energy: 0.80
- Candidate: 124 BPM, 8B key, 0.76 energy
- BPM: `1.0 - (2.0/12.0) = 0.833`
- Key: 8A vs 8B = same number, different letter = compatible → `1.0`
- Energy: `1.0 - (|0.76-0.80|/0.35) = 1.0 - 0.114 = 0.886`
- Final: `(0.833 * 0.36) + (1.0 * 0.34) + (0.886 * 0.30) = 0.300 + 0.340 + 0.266 = 0.906`

**Data in:** `anchor: &Track`, `candidate: &Track`, `target_energy: f32`
**Data out:** `f32` (0.0–1.0)

---

### Step 9: Camelot Wheel Key Compatibility — key_compatible

[VERIFIED: src-tauri/src/lib.rs:1144-1166]
```rust
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
```

Implements the Camelot wheel — the DJ-standard system for harmonic mixing. Three compatibility rules:

| Rule | Example | Musical Meaning |
|------|---------|----------------|
| Same key | 8A = 8A | Identical key |
| Same number, different letter | 8A ↔ 8B | Relative major/minor |
| Adjacent numbers, same letter | 7A ↔ 8A, 12A ↔ 1A | Perfect fifth / fourth |

The `diff == 11` check handles the wheel wrap-around (12A ↔ 1A = `|12-1| = 11`).

**Calls:** `split_key()` to parse Camelot notation

---

### Step 10: Key Parsing — split_key

[VERIFIED: src-tauri/src/lib.rs:1168-1173]
```rust
fn split_key(value: &str) -> Option<(u8, char)> {
    let split_at = value.len().checked_sub(1)?;
    let (number, letter) = value.split_at(split_at);
    Some((number.parse().ok()?, letter.chars().next()?))
}
```

Parses Camelot notation like "8A" into `(8, 'A')` or "12B" into `(12, 'B')`. Returns `None` if the string can't be parsed, causing `key_compatible` to return `false`.

**Data in:** `"8A"` (Camelot key string)
**Data out:** `Some((8, 'A'))` or `None`

---

### Step 11: Sort, Truncate, Assign Reasons

[VERIFIED: src-tauri/src/lib.rs:1070-1076]
```rust
scored.sort_by(|left, right| {
    right
        .compatibility
        .partial_cmp(&left.compatibility)
        .unwrap_or(Ordering::Equal)
});
scored.truncate(5);
```

Sorts candidates by compatibility score descending (highest first), then keeps only the top 5. The sort uses `partial_cmp` because `f32` doesn't implement `Ord` (NaN edge case).

**Data shape after truncation:** `Vec<SuggestionTrack>` with at most 5 entries, ordered by score.

---

### Step 12: Insight Generation

[VERIFIED: src-tauri/src/lib.rs:1078-1118]
```rust
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
```

Three possible insights, generated conditionally:

| Insight | Condition | Priority |
|---------|-----------|----------|
| **Energy arc** | Always | high |
| **Deck relationship** | Both decks loaded | medium |
| **Best cue strategy** | Always (static text) | low |

The "Energy arc" insight reports whether the target energy "climbing" or "breathing" relative to the anchor. The "Deck relationship" insight shows the BPM delta between the two loaded decks.

---

### Step 13: Payload Assembly

[VERIFIED: src-tauri/src/lib.rs:1120-1130]
```rust
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
```

**Data out:**
```rust
MixAssistantPayload {
    headline: "Best next move after {anchor.title}",
    summary: "Scored {N-1} library tracks by BPM distance, key compatibility, and target energy.",
    target_energy: f32,
    suggestions: Vec<SuggestionTrack>,  // max 5, sorted by compatibility desc
    insights: Vec<AssistantInsight>,    // 2-3 items
}
```

---

### Step 14: IPC Return

[VERIFIED: src-tauri/src/lib.rs:422]
```rust
Ok(payload)
```

The `MixAssistantPayload` is serialized via serde JSON through Tauri IPC back to the frontend. Note: unlike all other commands, this returns `MixAssistantPayload` directly — **not** `AppSnapshot`.

---

### Step 15: Frontend State Merge

[VERIFIED: src/App.tsx:123]
```tsx
setSnapshot((current) => (current ? { ...current, assistant } : current));
```

Uses a functional state update to merge only the `assistant` field into the existing snapshot. This preserves all other snapshot state (tracks, crates, stats, spotify) while updating just the assistant payload. This is the only place in the app where a partial snapshot update occurs — all other flows replace the entire snapshot via `applySnapshot()`.

[INFERRED: This partial update pattern avoids a full snapshot round-trip for what is a read-only query — `build_mix_assistant` doesn't mutate backend state]

---

### Step 16: AssistantPanel Render

[VERIFIED: src/App.tsx:406-412]
```tsx
<AssistantPanel
    assistant={snapshot?.assistant ?? null}
    onLoadSuggestion={(track) => loadDeck(track, "B")}
    onTargetEnergyChange={setTargetEnergy}
    targetEnergy={targetEnergy}
    trackMap={trackMap}
/>
```

[VERIFIED: src/components/AssistantPanel.tsx:11-89]
The AssistantPanel receives the updated `assistant` payload and renders:
- Headline and summary text
- Energy target slider (drives `targetEnergy` state)
- Up to 5 suggestion tracks with compatibility scores and reason text
- Set insights (energy arc, deck relationship, cue strategy)

Loading a suggestion calls `loadDeck(track, "B")` — always to Deck B — which in turn changes `deckB.state.track?.id`, re-triggering this entire flow (Step 1).

---

## External Calls

None. This flow is entirely in-memory computation. No filesystem, network, or database access.

---

## Events Fired

No events are dispatched. The flow is reactive through React's `useEffect` dependency array, not through an event system.

---

## Data Shape at Key Boundaries

### IPC Request (Frontend -> Backend)
```json
{
    "deck_a_track_id": "demo-cascade-nova",
    "deck_b_track_id": "demo-solar-rush",
    "target_energy": 0.78
}
```

### IPC Response (Backend -> Frontend)
```json
{
    "headline": "Best next move after Solar Rush",
    "summary": "Scored 7 library tracks by BPM distance, key compatibility, and target energy.",
    "target_energy": 0.78,
    "suggestions": [
        {
            "track_id": "demo-lux-prism",
            "reason": "Strong harmonic blend and tight tempo handoff",
            "compatibility": 0.91
        }
    ],
    "insights": [
        {
            "title": "Energy arc",
            "detail": "Current anchor is 76% energy. Targeting 78% keeps the room climbing.",
            "priority": "high"
        },
        {
            "title": "Deck relationship",
            "detail": "Deck A Cascade Nova (127.3 BPM / 5B) and Deck B Solar Rush (129.0 BPM / 8A) are 1.7 BPM apart.",
            "priority": "medium"
        },
        {
            "title": "Best cue strategy",
            "detail": "Aim the next mix around the Blend cue, then commit on the Drop marker if the room wants acceleration.",
            "priority": "low"
        }
    ]
}
```

---

## Reactive Feedback Loop

This flow creates a **feedback loop**: loading a suggestion into Deck B re-triggers the assistant, which generates new suggestions based on the newly loaded track as anchor. This is by design — the DJ continuously gets fresh recommendations as they work through their set.

```
[Load suggestion to Deck B]
        │
        ▼
  deckB.state.track.id changes
        │
        ▼
  useEffect fires (Step 1)
        │
        ▼
  New anchor = newly loaded track
        │
        ▼
  Fresh suggestions generated
        │
        ▼
  AssistantPanel updates
        │
        └── [DJ loads next suggestion] ──→ (loop)
```

---

## Also Triggered By: refresh_snapshot

[VERIFIED: src-tauri/src/lib.rs:619-624]
```rust
snapshot.assistant = generate_mix_assistant(
    &snapshot.tracks,
    deck_a_track_id,
    deck_b_track_id,
    target_energy,
);
```

`generate_mix_assistant` is also called inside `refresh_snapshot()`, which runs at the end of `scan_music_folder`, `import_spotify_library`, and `bootstrap_app`. In those contexts, deck IDs and energy are typically `None`, so the assistant uses `tracks[0]` as anchor and its energy as target. The frontend then immediately re-triggers Step 1 with actual deck state, overwriting this initial assistant payload.

---

## Known Issues Found

### 1. Anchor preference is inverted from typical DJ workflow
[VERIFIED: src-tauri/src/lib.rs:1041]
```rust
let anchor = deck_b.or(deck_a).unwrap_or(&tracks[0]);
```
Deck B is preferred as anchor. In typical DJ software, Deck B is the "incoming" track (what you're mixing into), so suggestions should be for what comes *after* Deck B. This is correct. However, if a DJ uses Deck A as their primary and Deck B as the cue deck, the anchor logic would be backwards. There's no user-configurable anchor preference.

### 2. Incompatible keys still get 0.42, not 0.0
[VERIFIED: src-tauri/src/lib.rs:1136-1138]
A key-incompatible track scores `0.42 * 0.34 = 0.143` for the key factor. With high BPM and energy matches, an incompatible-key track can still rank above a compatible-key track with slightly worse BPM/energy. This may surprise DJs who expect harmonic mixing to be a hard filter.

### 3. Reason text doesn't reflect the actual compatibility score
[VERIFIED: src-tauri/src/lib.rs:1050-1060]
The reason string is selected by simple threshold checks (key+BPM, energy delta, energy direction) that are independent of the `compatibility_score()` calculation. A track could score 0.95 from the weighted formula but get the generic "Recovery option" reason because its energy is slightly lower.

### 4. Static cue strategy insight
[VERIFIED: src-tauri/src/lib.rs:1112-1118]
The "Best cue strategy" insight is hardcoded static text. It doesn't reference the actual cue points of the anchor or suggested tracks.

### 5. No genre/tag consideration in scoring
[NOT_FOUND: searched "genre" and "tag" in generate_mix_assistant and compatibility_score]
Genre tags are not used in the compatibility calculation. Two tracks from completely different genres (e.g., hip-hop at 88 BPM and drum & bass at 88 BPM) could score identically if BPM, key, and energy align.

### 6. Linear scan for every trigger
[VERIFIED: src-tauri/src/lib.rs:1044-1068]
Every invocation iterates and scores all tracks in the library (`O(n)`). For the MVP scale this is fine, but with thousands of tracks and rapid slider movement (continuous `targetEnergy` changes), this could cause IPC congestion. The frontend's stale-closure `cancelled` flag mitigates stale responses but doesn't prevent the backend work.

### 7. Mutex used for read-only access
[VERIFIED: src-tauri/src/lib.rs:411-414]
```rust
let guard = state.0.lock().map_err(|_| String::from("State lock poisoned"))?;
```
`build_mix_assistant` only reads `snapshot.tracks` but acquires a full `Mutex` lock. An `RwLock` would allow concurrent assistant queries without blocking other commands. In practice this is rarely an issue since the frontend serializes assistant calls via the `cancelled` pattern.
