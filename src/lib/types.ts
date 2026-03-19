export type TrackSource = "local" | "spotify" | "hybrid";

export interface CuePoint {
  label: string;
  time_seconds: number;
  color: string;
}

export interface Track {
  id: string;
  source: TrackSource;
  path: string | null;
  spotify_id: string | null;
  title: string;
  artist: string;
  album: string;
  duration_seconds: number;
  bpm: number;
  musical_key: string;
  energy: number;
  genre_tags: string[];
  waveform: number[];
  cue_points: CuePoint[];
  imported_from: string | null;
  year: number | null;
  artwork_url: string | null;
  stem_parent_id: string | null;
  stem_type: string | null;
  stem_ids: string[];
}

export interface StemProgressEvent {
  track_id: string;
  percent: number;
  stage: string;
}

export interface Crate {
  id: string;
  name: string;
  color: string;
  icon: string;
  description: string;
  source: string;
  track_ids: string[];
}

export interface SpotifyConnection {
  client_id: string | null;
  redirect_uri: string | null;
  access_token_present: boolean;
  last_sync_summary: string;
  last_import_mode: string;
  recommended_scopes: string[];
}

export interface LibraryStats {
  total_tracks: number;
  local_tracks: number;
  spotify_tracks: number;
  hybrid_tracks: number;
  avg_bpm: number;
  avg_energy: number;
  genres: number;
}

export interface AssistantInsight {
  title: string;
  detail: string;
  priority: string;
}

export interface SuggestionTrack {
  track_id: string;
  reason: string;
  compatibility: number;
}

export interface MixAssistantPayload {
  headline: string;
  summary: string;
  target_energy: number;
  suggestions: SuggestionTrack[];
  insights: AssistantInsight[];
}

export interface AppSnapshot {
  tracks: Track[];
  crates: Crate[];
  spotify: SpotifyConnection;
  stats: LibraryStats;
  assistant: MixAssistantPayload;
  last_scan_path: string | null;
  status: string;
}

export interface SpotifyConfigInput {
  client_id?: string | null;
  redirect_uri?: string | null;
  access_token?: string | null;
}

