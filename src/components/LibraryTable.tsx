import { useState } from "react";

import type { StemProgressEvent, Track } from "../lib/types";

export type SortKey = "title" | "artist" | "bpm" | "energy" | "key";

interface LibraryTableProps {
  deckATrackId: string | null;
  deckBTrackId: string | null;
  onLoadDeckA: (track: Track) => void;
  onLoadDeckB: (track: Track) => void;
  onSelectTrack: (trackId: string) => void;
  onSeparateStems: (trackId: string, stemCount: number) => void;
  selectedTrackId: string | null;
  sortKey: SortKey;
  stemProgress: Map<string, StemProgressEvent>;
  stemsReady: boolean;
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

const STEM_BADGE_COLORS: Record<string, string> = {
  vocals: "#ff7aa6",
  drums: "#ff7a18",
  bass: "#29f0b4",
  other: "#88a8ff",
  no_vocals: "#ffcd6f",
};

export function LibraryTable({
  deckATrackId,
  deckBTrackId,
  onLoadDeckA,
  onLoadDeckB,
  onSelectTrack,
  onSeparateStems,
  selectedTrackId,
  sortKey,
  stemProgress,
  stemsReady,
  tracks,
}: LibraryTableProps) {
  const [expandedStems, setExpandedStems] = useState<Set<string>>(new Set());
  const [stemMenuOpen, setStemMenuOpen] = useState<string | null>(null);

  function toggleStems(trackId: string) {
    setExpandedStems((current) => {
      const next = new Set(current);
      if (next.has(trackId)) {
        next.delete(trackId);
      } else {
        next.add(trackId);
      }
      return next;
    });
  }

  const stemTrackMap = new Map<string, Track[]>();
  const parentTracks: Track[] = [];

  for (const track of tracks) {
    if (track.stem_parent_id) {
      const siblings = stemTrackMap.get(track.stem_parent_id) ?? [];
      siblings.push(track);
      stemTrackMap.set(track.stem_parent_id, siblings);
    } else {
      parentTracks.push(track);
    }
  }

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
            {parentTracks.map((track) => {
              const inDeckA = track.id === deckATrackId;
              const inDeckB = track.id === deckBTrackId;
              const progress = stemProgress.get(track.id);
              const hasStemChildren = track.stem_ids.length > 0;
              const isExpanded = expandedStems.has(track.id);
              const canSeparate = stemsReady && track.path !== null && !progress;
              const childStems = stemTrackMap.get(track.id) ?? [];

              return [
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

                      {hasStemChildren ? (
                        <button
                          className="stem-toggle"
                          onClick={(event) => {
                            event.stopPropagation();
                            toggleStems(track.id);
                          }}
                          type="button"
                        >
                          {isExpanded ? "▾" : "▸"} Stems
                        </button>
                      ) : canSeparate ? (
                        <span className="stem-actions">
                          <button
                            className="stem-trigger"
                            onClick={(event) => {
                              event.stopPropagation();
                              setStemMenuOpen(stemMenuOpen === track.id ? null : track.id);
                            }}
                            type="button"
                          >
                            Stems
                          </button>
                          {stemMenuOpen === track.id && (
                            <span className="stem-menu">
                              <button
                                onClick={(event) => {
                                  event.stopPropagation();
                                  setStemMenuOpen(null);
                                  onSeparateStems(track.id, 2);
                                }}
                                type="button"
                              >
                                2-stem
                              </button>
                              <button
                                onClick={(event) => {
                                  event.stopPropagation();
                                  setStemMenuOpen(null);
                                  onSeparateStems(track.id, 4);
                                }}
                                type="button"
                              >
                                4-stem
                              </button>
                            </span>
                          )}
                        </span>
                      ) : progress ? (
                        <span className="stem-progress">
                          {Math.round(progress.percent)}%
                        </span>
                      ) : null}
                    </div>
                  </td>
                </tr>,

                ...(isExpanded
                  ? childStems.map((stem) => (
                      <tr
                        key={stem.id}
                        className={[
                          "stem-row",
                          stem.id === deckATrackId ? "in-deck-a" : "",
                          stem.id === deckBTrackId ? "in-deck-b" : "",
                        ]
                          .filter(Boolean)
                          .join(" ")}
                        onClick={() => onSelectTrack(stem.id)}
                      >
                        <td>
                          <div className="track-cell stem-cell">
                            <span
                              className="stem-badge"
                              style={{
                                borderColor: STEM_BADGE_COLORS[stem.stem_type ?? ""] ?? "#888",
                                color: STEM_BADGE_COLORS[stem.stem_type ?? ""] ?? "#888",
                              }}
                            >
                              {(stem.stem_type ?? "stem").toUpperCase()}
                            </span>
                            <strong>{stem.title}</strong>
                          </div>
                        </td>
                        <td />
                        <td>{stem.bpm.toFixed(1)}</td>
                        <td>{stem.musical_key}</td>
                        <td>{Math.round(stem.energy * 100)}%</td>
                        <td>{formatDuration(stem.duration_seconds)}</td>
                        <td>stem</td>
                        <td>
                          <div className="load-actions">
                            <button onClick={() => onLoadDeckA(stem)} type="button">
                              A
                            </button>
                            <button onClick={() => onLoadDeckB(stem)} type="button">
                              B
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))
                  : []),
              ];
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
