import { settingsStore } from "$lib/stores/settings.svelte";

let ctx: AudioContext | null = null;

function audio(): AudioContext | null {
  if (typeof window === "undefined") return null;
  try {
    ctx ??= new (window.AudioContext ?? (window as any).webkitAudioContext)();
    return ctx;
  } catch {
    return null;
  }
}

export function playClick(volume = 0.12) {
  const scaled = volume * ((settingsStore.settings.soundVolume ?? 100) / 100);
  if (scaled <= 0) return;
  const ac = audio();
  if (!ac) return;
  // A click is a user gesture, so resuming here is allowed.
  if (ac.state === "suspended") ac.resume();

  const now = ac.currentTime;
  const osc = ac.createOscillator();
  const gain = ac.createGain();

  osc.type = "triangle";
  osc.frequency.setValueAtTime(680, now);
  osc.frequency.exponentialRampToValueAtTime(430, now + 0.05);

  gain.gain.setValueAtTime(0.0001, now);
  gain.gain.exponentialRampToValueAtTime(scaled, now + 0.005);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.06);

  osc.connect(gain).connect(ac.destination);
  osc.start(now);
  osc.stop(now + 0.07);
}
