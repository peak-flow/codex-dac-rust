import type { Track } from "../lib/types";

export type SortKey = "title" | "artist" | "bpm" | "energy" | "key";

interface LibraryTableProps {
  deckATrackId: string | null;
  deckBTrackId: string | null;
  onLoadDeckA: (track: Track) => void;
  onLoadDeckB: (track: Track) => void;
  onSelectTrack: (trackId: string) => void;
  selectedTrackId: string | null;
  sortKey: SortKey;
  tracks: Track[];
}

function formatDuration(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return "--:--";
  }

  const minutes = Math.floor(value / 60);
  const seconds = Math.floor(value % 60)
    .toString()
    .padStart(2, "0");

  return `${minutes}:${seconds}`;
}

export function LibraryTable({
  deckATrackId,
  deckBTrackId,
  onLoadDeckA,
  onLoadDeckB,
  onSelectTrack,
  selectedTrackId,
  sortKey,
  tracks,
}: LibraryTableProps) {
  return (
    <div className="panel library-panel">
      <div className="table-header">
        <div>
          <p className="eyebrow">Library Browser</p>
          <h3>Searchable, sortable, deck-ready track view</h3>
        </div>
        <span className="table-pill">Sorted by {sortKey.toUpperCase()}</span>
      </div>

      <div className="library-table-wrap">
        <table className="library-table">
          <thead>
            <tr>
              <th>Track</th>
              <th>Album</th>
              <th>BPM</th>
              <th>Key</th>
              <th>Energy</th>
              <th>Length</th>
              <th>Source</th>
              <th>Load</th>
            </tr>
          </thead>
          <tbody>
            {tracks.map((track) => {
              const inDeckA = track.id === deckATrackId;
              const inDeckB = track.id === deckBTrackId;

              return (
                <tr
                  key={track.id}
                  className={[
                    selectedTrackId === track.id ? "is-selected" : "",
                    inDeckA ? "in-deck-a" : "",
                    inDeckB ? "in-deck-b" : "",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() => onSelectTrack(track.id)}
                >
                  <td>
                    <div className="track-cell">
                      <strong>{track.title}</strong>
                      <span>
                        {track.artist}
                        {track.genre_tags.length > 0 ? ` • ${track.genre_tags.slice(0, 2).join(" / ")}` : ""}
                      </span>
                    </div>
                  </td>
                  <td>{track.album}</td>
                  <td>{track.bpm.toFixed(1)}</td>
                  <td>{track.musical_key}</td>
                  <td>{Math.round(track.energy * 100)}%</td>
                  <td>{formatDuration(track.duration_seconds)}</td>
                  <td>{track.source}</td>
                  <td>
                    <div className="load-actions">
                      <button onClick={() => onLoadDeckA(track)} type="button">
                        A
                      </button>
                      <button onClick={() => onLoadDeckB(track)} type="button">
                        B
                      </button>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

