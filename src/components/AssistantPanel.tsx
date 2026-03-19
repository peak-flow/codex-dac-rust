import type { MixAssistantPayload, Track } from "../lib/types";

interface AssistantPanelProps {
  assistant: MixAssistantPayload | null;
  onLoadSuggestion: (track: Track) => void;
  onTargetEnergyChange: (value: number) => void;
  targetEnergy: number;
  trackMap: Map<string, Track>;
}

export function AssistantPanel({
  assistant,
  onLoadSuggestion,
  onTargetEnergyChange,
  targetEnergy,
  trackMap,
}: AssistantPanelProps) {
  return (
    <section className="panel assistant-panel">
      <div className="assistant-header">
        <div>
          <p className="eyebrow">AI Set Architect</p>
          <h3>{assistant?.headline ?? "Analyzing flow..."}</h3>
          <p>{assistant?.summary ?? "Collecting energy, key, and transition context from the library."}</p>
        </div>

        <label className="energy-dial">
          <span>Target energy</span>
          <input
            max={1}
            min={0.35}
            onChange={(event) => onTargetEnergyChange(Number(event.target.value))}
            step={0.01}
            type="range"
            value={targetEnergy}
          />
          <strong>{Math.round(targetEnergy * 100)}%</strong>
        </label>
      </div>

      <div className="assistant-grid">
        <div className="assistant-card">
          <p className="eyebrow">Next Best Tracks</p>
          <div className="assistant-list">
            {(assistant?.suggestions ?? []).map((suggestion) => {
              const track = trackMap.get(suggestion.track_id);

              if (!track) {
                return null;
              }

              return (
                <button
                  key={suggestion.track_id}
                  className="assistant-track"
                  onClick={() => onLoadSuggestion(track)}
                  type="button"
                >
                  <div>
                    <strong>{track.title}</strong>
                    <span>
                      {track.artist} • {track.bpm.toFixed(1)} BPM • {track.musical_key}
                    </span>
                  </div>
                  <div className="assistant-score">
                    <em>{Math.round(suggestion.compatibility * 100)}%</em>
                    <span>{suggestion.reason}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        <div className="assistant-card">
          <p className="eyebrow">Set Advice</p>
          <div className="insight-list">
            {(assistant?.insights ?? []).map((insight) => (
              <article key={insight.title} className={`insight-block priority-${insight.priority}`}>
                <strong>{insight.title}</strong>
                <p>{insight.detail}</p>
              </article>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

