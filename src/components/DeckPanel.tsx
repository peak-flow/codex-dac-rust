import type { RefObject } from "react";

import type { CuePoint, Track } from "../lib/types";
import type { DeckState } from "../hooks/useDeck";

function formatClock(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return "00:00";
  }

  const minutes = Math.floor(value / 60);
  const seconds = Math.floor(value % 60)
    .toString()
    .padStart(2, "0");

  return `${minutes.toString().padStart(2, "0")}:${seconds}`;
}

function describeSource(track: Track | null) {
  if (!track) {
    return "EMPTY";
  }

  if (track.path) {
    return "LOCAL AUDIO";
  }

  if (track.source === "spotify") {
    return "SPOTIFY METADATA";
  }

  if (track.source === "hybrid") {
    return "HYBRID CRATE";
  }

  return "METADATA ONLY";
}

interface DeckPanelProps {
  accent: string;
  audioRef: RefObject<HTMLAudioElement | null>;
  deck: DeckState;
  canSync: boolean;
  onEject: () => void;
  onJumpToCue: (cue: CuePoint) => void;
  onNudge: (seconds: number) => void;
  onSeek: (seconds: number) => void;
  onSetTempo: (rate: number) => void;
  onSetVolume: (volume: number) => void;
  onStop: () => void;
  onSync: () => void;
  onTogglePlay: () => void | Promise<void>;
  statusText: string;
}

export function DeckPanel({
  accent,
  audioRef,
  deck,
  canSync,
  onEject,
  onJumpToCue,
  onNudge,
  onSeek,
  onSetTempo,
  onSetVolume,
  onStop,
  onSync,
  onTogglePlay,
  statusText,
}: DeckPanelProps) {
  const duration = deck.display_duration || deck.track?.duration_seconds || 0;
  const progress = duration > 0 ? deck.current_time / duration : 0;
  const waveform = deck.track?.waveform ?? Array.from({ length: 72 }, (_, index) => 0.12 + ((index % 5) + 1) * 0.08);

  return (
    <section className={`deck-panel deck-${deck.deck_id.toLowerCase()}`}>
      <audio ref={audioRef} preload="metadata" />

      <div className="deck-topline">
        <div>
          <p className="deck-label">Deck {deck.deck_id}</p>
          <h2>{deck.track?.title ?? "Drag a track in"}</h2>
          <p className="deck-subtitle">
            {deck.track?.artist ?? "Load from library, smart suggestions, or Spotify import"}
          </p>
        </div>

        <div className="deck-chip-stack">
          <span className="deck-chip" style={{ borderColor: accent }}>
            {describeSource(deck.track)}
          </span>
          <span className="deck-chip">{statusText}</span>
        </div>
      </div>

      <div className="deck-metrics">
        <div className="metric-card">
          <span>BPM</span>
          <strong>{deck.track ? deck.track.bpm.toFixed(1) : "--"}</strong>
        </div>
        <div className="metric-card">
          <span>KEY</span>
          <strong>{deck.track?.musical_key ?? "--"}</strong>
        </div>
        <div className="metric-card">
          <span>ENERGY</span>
          <strong>{deck.track ? `${Math.round(deck.track.energy * 100)}%` : "--"}</strong>
        </div>
        <div className="metric-card">
          <span>PITCH</span>
          <strong>{deck.track ? `${deck.pitch_percent >= 0 ? "+" : ""}${deck.pitch_percent.toFixed(1)}%` : "--"}</strong>
        </div>
      </div>

      <div className="wave-shell">
        <div className="wave-grid">
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
        </div>

        <div className="wave-readout">
          <span>{formatClock(deck.current_time)}</span>
          <div className="progress-track">
            <div className="progress-fill" style={{ width: `${Math.min(progress * 100, 100)}%`, background: accent }} />
          </div>
          <span>{formatClock(duration)}</span>
        </div>
      </div>

      <div className="transport-strip">
        <button className="transport ghost" onClick={() => onNudge(-8)} type="button">
          -8
        </button>
        <button className="transport" onClick={() => void onTogglePlay()} type="button">
          {deck.playing ? "Pause" : "Play"}
        </button>
        <button className="transport" onClick={onStop} type="button">
          Stop
        </button>
        <button className="transport ghost" onClick={() => onNudge(8)} type="button">
          +8
        </button>
        <button className="transport sync" disabled={!canSync} onClick={onSync} type="button">
          Beat Sync
        </button>
        <button className="transport ghost" disabled={!deck.track} onClick={onEject} type="button">
          Eject
        </button>
      </div>

      <div className="deck-sliders">
        <label className="slider-block">
          <span>Tempo</span>
          <input
            max={1.22}
            min={0.78}
            onChange={(event) => onSetTempo(Number(event.target.value))}
            step={0.005}
            type="range"
            value={deck.rate}
          />
          <em>{deck.rate.toFixed(3)}x</em>
        </label>

        <label className="slider-block">
          <span>Volume</span>
          <input
            max={1}
            min={0}
            onChange={(event) => onSetVolume(Number(event.target.value))}
            step={0.01}
            type="range"
            value={deck.volume}
          />
          <em>{Math.round(deck.volume * 100)}%</em>
        </label>
      </div>

      <div className="cue-bank">
        {(deck.track?.cue_points ?? []).map((cue) => (
          <button
            key={`${deck.deck_id}-${cue.label}`}
            className="cue-chip"
            onClick={() => onJumpToCue(cue)}
            style={{ borderColor: cue.color, color: cue.color }}
            type="button"
          >
            {cue.label}
            <span>{formatClock(cue.time_seconds)}</span>
          </button>
        ))}
      </div>
    </section>
  );
}

