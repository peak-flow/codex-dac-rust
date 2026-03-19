import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useEffectEvent, useRef, useState } from "react";

import type { CuePoint, Track } from "../lib/types";

export interface DeckState {
  deck_id: "A" | "B";
  track: Track | null;
  playing: boolean;
  current_time: number;
  display_duration: number;
  volume: number;
  rate: number;
  pitch_percent: number;
}

function createInitialState(deckId: "A" | "B"): DeckState {
  return {
    deck_id: deckId,
    track: null,
    playing: false,
    current_time: 0,
    display_duration: 0,
    volume: deckId === "A" ? 0.92 : 0.88,
    rate: 1,
    pitch_percent: 0,
  };
}

export function useDeck(deckId: "A" | "B") {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [state, setState] = useState<DeckState>(() => createInitialState(deckId));

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

  const handleEnded = useEffectEvent(() => {
    setState((current) => ({
      ...current,
      playing: false,
      current_time: 0,
    }));
  });

  useEffect(() => {
    const audio = audioRef.current;

    if (!audio) {
      return;
    }

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

  useEffect(() => {
    const audio = audioRef.current;

    if (!audio) {
      return;
    }

    audio.volume = state.volume;
    audio.playbackRate = state.rate;
  }, [state.volume, state.rate]);

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

  function setVolume(volume: number) {
    const clamped = Math.max(0, Math.min(volume, 1));

    setState((current) => ({
      ...current,
      volume: clamped,
    }));
  }

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

  function nudge(seconds: number) {
    seek(state.current_time + seconds);
  }

  return {
    audioRef,
    state,
    loadTrack,
    ejectTrack,
    stop,
    seek,
    jumpToCue,
    togglePlay,
    setVolume,
    setTempo,
    nudge,
  };
}

