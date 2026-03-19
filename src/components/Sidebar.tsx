import type { Crate, LibraryStats, SpotifyConnection } from "../lib/types";

interface SidebarProps {
  crates: Crate[];
  selectedCrateId: string;
  stats: LibraryStats;
  spotify: SpotifyConnection;
  onSelectCrate: (crateId: string) => void;
}

export function Sidebar({
  crates,
  selectedCrateId,
  stats,
  spotify,
  onSelectCrate,
}: SidebarProps) {
  const smartCrates = crates.filter((crate) => crate.source === "smart");
  const importedCrates = crates.filter((crate) => crate.source !== "smart");

  return (
    <aside className="sidebar">
      <section className="panel hero-panel">
        <p className="eyebrow">PulseGrid DJ</p>
        <h1>Industrial mixing control, with a set-planning copilot built in.</h1>
        <p className="hero-copy">
          Scan local folders, blend imported Spotify metadata, and let the assistant steer energy,
          key, and transition timing.
        </p>

        <div className="stats-grid">
          <div>
            <span>Total Tracks</span>
            <strong>{stats.total_tracks}</strong>
          </div>
          <div>
            <span>Avg BPM</span>
            <strong>{stats.avg_bpm.toFixed(1)}</strong>
          </div>
          <div>
            <span>Avg Energy</span>
            <strong>{Math.round(stats.avg_energy * 100)}%</strong>
          </div>
          <div>
            <span>Genres</span>
            <strong>{stats.genres}</strong>
          </div>
        </div>

        <div className="status-rail">
          <span className={`status-pill ${spotify.access_token_present ? "live" : ""}`}>
            {spotify.access_token_present ? "Spotify token loaded" : "Spotify token needed"}
          </span>
          <span className="status-pill">{spotify.last_import_mode.replace(/_/g, " ")}</span>
        </div>
      </section>

      <section className="panel crate-panel">
        <div className="section-heading">
          <p className="eyebrow">Smart Crates</p>
          <span>{smartCrates.length}</span>
        </div>

        <button
          className={`crate-row ${selectedCrateId === "all" ? "is-selected" : ""}`}
          onClick={() => onSelectCrate("all")}
          type="button"
        >
          <span className="crate-icon">ALL</span>
          <div>
            <strong>Unified Library</strong>
            <span>Every scanned and imported track</span>
          </div>
        </button>

        {smartCrates.map((crate) => (
          <button
            key={crate.id}
            className={`crate-row ${selectedCrateId === crate.id ? "is-selected" : ""}`}
            onClick={() => onSelectCrate(crate.id)}
            type="button"
          >
            <span className="crate-icon" style={{ color: crate.color }}>
              {crate.icon}
            </span>
            <div>
              <strong>{crate.name}</strong>
              <span>{crate.description}</span>
            </div>
          </button>
        ))}
      </section>

      <section className="panel crate-panel">
        <div className="section-heading">
          <p className="eyebrow">Imported Crates</p>
          <span>{importedCrates.length}</span>
        </div>

        {importedCrates.map((crate) => (
          <button
            key={crate.id}
            className={`crate-row ${selectedCrateId === crate.id ? "is-selected" : ""}`}
            onClick={() => onSelectCrate(crate.id)}
            type="button"
          >
            <span className="crate-icon" style={{ color: crate.color }}>
              {crate.icon}
            </span>
            <div>
              <strong>{crate.name}</strong>
              <span>{crate.track_ids.length} tracks</span>
            </div>
          </button>
        ))}
      </section>
    </aside>
  );
}

