import { invoke } from "@tauri-apps/api/core";

import type { AppSnapshot, MixAssistantPayload, SpotifyConfigInput } from "./types";

export function bootstrapApp() {
  return invoke<AppSnapshot>("bootstrap_app");
}

export function saveSpotifyConfig(config: SpotifyConfigInput) {
  return invoke<AppSnapshot>("save_spotify_config", { config });
}

export function importSpotifyLibrary(accessToken?: string) {
  return invoke<AppSnapshot>("import_spotify_library", {
    access_token: accessToken ?? null,
  });
}

export function scanMusicFolder(path: string) {
  return invoke<AppSnapshot>("scan_music_folder", { path });
}

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
