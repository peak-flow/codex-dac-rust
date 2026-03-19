import { open } from "@tauri-apps/plugin-dialog";
import { startTransition, useDeferredValue, useEffect, useState } from "react";

import { AssistantPanel } from "./components/AssistantPanel";
import { DeckPanel } from "./components/DeckPanel";
import { LibraryTable, type SortKey } from "./components/LibraryTable";
import { Sidebar } from "./components/Sidebar";
import { useDeck } from "./hooks/useDeck";
import {
  bootstrapApp,
  buildMixAssistant,
  importSpotifyLibrary,
  saveSpotifyConfig,
  scanMusicFolder,
} from "./lib/tauri";
import type { AppSnapshot, Track } from "./lib/types";

function normalize(value: string) {
  return value.trim().toLowerCase();
}

function sortTracks(tracks: Track[], sortKey: SortKey) {
  const sorted = [...tracks];

  sorted.sort((left, right) => {
    switch (sortKey) {
      case "artist":
        return left.artist.localeCompare(right.artist);
      case "bpm":
        return right.bpm - left.bpm;
      case "energy":
        return right.energy - left.energy;
      case "key":
        return left.musical_key.localeCompare(right.musical_key);
      case "title":
      default:
        return left.title.localeCompare(right.title);
    }
  });

  return sorted;
}

export default function App() {
  const deckA = useDeck("A");
  const deckB = useDeck("B");

  const [snapshot, setSnapshot] = useState<AppSnapshot | null>(null);
  const [selectedCrateId, setSelectedCrateId] = useState("all");
  const [selectedTrackId, setSelectedTrackId] = useState<string | null>(null);
  const [statusLine, setStatusLine] = useState("Powering up audio engine...");
  const [sortKey, setSortKey] = useState<SortKey>("energy");
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search);
  const [genreFilter, setGenreFilter] = useState("all");
  const [targetEnergy, setTargetEnergy] = useState(0.72);
  const [spotifyClientId, setSpotifyClientId] = useState("");
  const [spotifyRedirectUri, setSpotifyRedirectUri] = useState("http://127.0.0.1:8888/callback");
  const [spotifyAccessToken, setSpotifyAccessToken] = useState("");

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

  useEffect(() => {
    let cancelled = false;

    void (async () => {
      try {
        const initial = await bootstrapApp();

        if (cancelled) {
          return;
        }

        applySnapshot(initial);
        setTargetEnergy(initial.assistant.target_energy);
      } catch (error) {
        console.error(error);
        setStatusLine("Unable to bootstrap the DJ library.");
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

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

  async function handleChooseFolder() {
    const picked = await open({
      directory: true,
      multiple: false,
      title: "Choose a music folder",
    });

    if (typeof picked !== "string") {
      return;
    }

    setStatusLine(`Scanning ${picked} ...`);

    try {
      const next = await scanMusicFolder(picked);
      applySnapshot(next);
    } catch (error) {
      console.error(error);
      setStatusLine("Folder scan failed.");
    }
  }

  async function handleSaveSpotify() {
    try {
      const next = await saveSpotifyConfig({
        client_id: spotifyClientId || null,
        redirect_uri: spotifyRedirectUri || null,
        access_token: spotifyAccessToken || null,
      });

      applySnapshot(next);
      setStatusLine("Spotify credentials staged for import.");
    } catch (error) {
      console.error(error);
      setStatusLine("Could not save Spotify settings.");
    }
  }

  async function handleImportSpotify() {
    setStatusLine("Pulling playlists and saved tracks from Spotify...");

    try {
      const next = await importSpotifyLibrary(spotifyAccessToken || undefined);
      applySnapshot(next);
    } catch (error) {
      console.error(error);
      setStatusLine("Spotify import failed.");
    }
  }

  function loadDeck(track: Track, deckId: "A" | "B") {
    if (deckId === "A") {
      deckA.loadTrack(track);
    } else {
      deckB.loadTrack(track);
    }

    setSelectedTrackId(track.id);
    setStatusLine(`${track.title} loaded to Deck ${deckId}.`);
  }

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

  const trackMap = new Map(snapshot?.tracks.map((track) => [track.id, track]) ?? []);
  const genres = Array.from(
    new Set(
      (snapshot?.tracks ?? [])
        .flatMap((track) => track.genre_tags)
        .filter(Boolean),
    ),
  ).sort();

  const activeCrate = snapshot?.crates.find((crateItem) => crateItem.id === selectedCrateId) ?? null;
  const visibleTracks = sortTracks(
    (snapshot?.tracks ?? []).filter((track) => {
      const query = normalize(deferredSearch);
      const inCrate = !activeCrate || activeCrate.track_ids.includes(track.id);
      const matchesSearch = !query || normalize(`${track.title} ${track.artist} ${track.album} ${track.genre_tags.join(" ")}`).includes(query);
      const matchesGenre = genreFilter === "all" || track.genre_tags.includes(genreFilter);

      return inCrate && matchesSearch && matchesGenre;
    }),
    sortKey,
  );

  return (
    <div className="app-shell">
      <Sidebar
        crates={snapshot?.crates ?? []}
        onSelectCrate={setSelectedCrateId}
        selectedCrateId={selectedCrateId}
        spotify={snapshot?.spotify ?? {
          access_token_present: false,
          client_id: null,
          last_import_mode: "demo",
          last_sync_summary: "",
          recommended_scopes: [],
          redirect_uri: null,
        }}
        stats={snapshot?.stats ?? {
          avg_bpm: 0,
          avg_energy: 0,
          genres: 0,
          hybrid_tracks: 0,
          local_tracks: 0,
          spotify_tracks: 0,
          total_tracks: 0,
        }}
      />

      <main className="workspace">
        <section className="panel control-panel">
          <div className="control-cluster">
            <div className="control-card">
              <p className="eyebrow">Library Ingest</p>
              <h3>Scan a local music folder</h3>
              <p>
                Pull local files into the unified library, generate stable BPM/key/energy estimates,
                and auto-build smart crates.
              </p>
              <button className="primary-action" onClick={() => void handleChooseFolder()} type="button">
                Scan Music Folder
              </button>
              <span className="micro-copy">{snapshot?.last_scan_path ?? "No folder scanned yet"}</span>
            </div>

            <div className="control-card spotify-card">
              <p className="eyebrow">Spotify Bridge</p>
              <h3>Playlist + library metadata import</h3>
              <p>{snapshot?.spotify.last_sync_summary ?? "Load an access token, then import playlists and liked tracks."}</p>

              <div className="form-grid">
                <label>
                  Client ID
                  <input
                    onChange={(event) => setSpotifyClientId(event.target.value)}
                    placeholder="Optional for future OAuth flow"
                    type="text"
                    value={spotifyClientId}
                  />
                </label>
                <label>
                  Redirect URI
                  <input
                    onChange={(event) => setSpotifyRedirectUri(event.target.value)}
                    type="text"
                    value={spotifyRedirectUri}
                  />
                </label>
                <label className="full-span">
                  Access Token
                  <input
                    onChange={(event) => setSpotifyAccessToken(event.target.value)}
                    placeholder="Paste a user token with playlist-read-private, playlist-read-collaborative, user-library-read"
                    type="password"
                    value={spotifyAccessToken}
                  />
                </label>
              </div>

              <div className="button-row">
                <button className="secondary-action" onClick={() => void handleSaveSpotify()} type="button">
                  Save Credentials
                </button>
                <button className="primary-action" onClick={() => void handleImportSpotify()} type="button">
                  Import Spotify Library
                </button>
              </div>
            </div>

            <div className="control-card">
              <p className="eyebrow">Live Ops</p>
              <h3>Filter, sort, and prep for decks</h3>
              <p>{statusLine}</p>

              <div className="filter-grid">
                <label>
                  Search
                  <input
                    onChange={(event) => setSearch(event.target.value)}
                    placeholder="title, artist, album, tag"
                    type="search"
                    value={search}
                  />
                </label>
                <label>
                  Genre
                  <select onChange={(event) => setGenreFilter(event.target.value)} value={genreFilter}>
                    <option value="all">All genres</option>
                    {genres.map((genre) => (
                      <option key={genre} value={genre}>
                        {genre}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Sort
                  <select onChange={(event) => setSortKey(event.target.value as SortKey)} value={sortKey}>
                    <option value="energy">Energy</option>
                    <option value="bpm">BPM</option>
                    <option value="key">Key</option>
                    <option value="title">Title</option>
                    <option value="artist">Artist</option>
                  </select>
                </label>
              </div>
            </div>
          </div>
        </section>

        <section className="deck-grid">
          <DeckPanel
            accent="#ff7a18"
            audioRef={deckA.audioRef}
            canSync={Boolean(deckA.state.track && deckB.state.track)}
            deck={deckA.state}
            onEject={deckA.ejectTrack}
            onJumpToCue={deckA.jumpToCue}
            onNudge={deckA.nudge}
            onSeek={deckA.seek}
            onSetTempo={deckA.setTempo}
            onSetVolume={deckA.setVolume}
            onStop={deckA.stop}
            onSync={() => handleSync("A")}
            onTogglePlay={deckA.togglePlay}
            statusText={deckA.state.track?.path ? "Audio armed" : "Visual mode"}
          />

          <DeckPanel
            accent="#29f0b4"
            audioRef={deckB.audioRef}
            canSync={Boolean(deckA.state.track && deckB.state.track)}
            deck={deckB.state}
            onEject={deckB.ejectTrack}
            onJumpToCue={deckB.jumpToCue}
            onNudge={deckB.nudge}
            onSeek={deckB.seek}
            onSetTempo={deckB.setTempo}
            onSetVolume={deckB.setVolume}
            onStop={deckB.stop}
            onSync={() => handleSync("B")}
            onTogglePlay={deckB.togglePlay}
            statusText={deckB.state.track?.path ? "Audio armed" : "Visual mode"}
          />
        </section>

        <section className="lower-grid">
          <div className="lower-stack">
            <LibraryTable
              deckATrackId={deckA.state.track?.id ?? null}
              deckBTrackId={deckB.state.track?.id ?? null}
              onLoadDeckA={(track) => loadDeck(track, "A")}
              onLoadDeckB={(track) => loadDeck(track, "B")}
              onSelectTrack={setSelectedTrackId}
              selectedTrackId={selectedTrackId}
              sortKey={sortKey}
              tracks={visibleTracks}
            />
          </div>

          <AssistantPanel
            assistant={snapshot?.assistant ?? null}
            onLoadSuggestion={(track) => loadDeck(track, "B")}
            onTargetEnergyChange={setTargetEnergy}
            targetEnergy={targetEnergy}
            trackMap={trackMap}
          />
        </section>
      </main>
    </div>
  );
}

