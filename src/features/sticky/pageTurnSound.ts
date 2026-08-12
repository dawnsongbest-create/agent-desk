type AudioWindow = Window & {
  webkitAudioContext?: typeof AudioContext;
};

let audioContext: AudioContext | null = null;
let activeSource: AudioBufferSourceNode | null = null;

function getAudioContext() {
  const AudioContextConstructor = window.AudioContext ?? (window as AudioWindow).webkitAudioContext;
  if (!AudioContextConstructor) return null;
  audioContext ??= new AudioContextConstructor();
  return audioContext;
}

/**
 * Creates a short paper-turn texture at runtime. This is original project code,
 * so no third-party audio recording or external asset license is involved.
 */
export function playPageTurnSound(direction: "note-to-todo" | "todo-to-note") {
  try {
    const context = getAudioContext();
    if (!context) return;

    if (activeSource) {
      try {
        activeSource.stop();
      } catch {
        // The previous paper texture may already have ended between rapid turns.
      }
      activeSource = null;
    }
    const duration = 0.34;
    const frameCount = Math.floor(context.sampleRate * duration);
    const buffer = context.createBuffer(1, frameCount, context.sampleRate);
    const channel = buffer.getChannelData(0);
    let previous = 0;

    for (let frame = 0; frame < frameCount; frame += 1) {
      const progress = frame / frameCount;
      const attack = Math.min(1, progress / 0.08);
      const release = Math.pow(1 - progress, 1.7);
      const sweep = direction === "note-to-todo" ? 0.86 + progress * 0.14 : 1 - progress * 0.14;
      const noise = Math.random() * 2 - 1;
      previous = previous * 0.62 + noise * 0.38;
      const fibre = Math.sin(progress * progress * 290) * 0.12;
      channel[frame] = (previous + fibre) * attack * release * sweep;
    }

    const source = context.createBufferSource();
    const filter = context.createBiquadFilter();
    const gain = context.createGain();
    const now = context.currentTime;

    source.buffer = buffer;
    filter.type = "bandpass";
    filter.Q.value = 0.55;
    filter.frequency.setValueAtTime(direction === "note-to-todo" ? 2200 : 1850, now);
    filter.frequency.exponentialRampToValueAtTime(
      direction === "note-to-todo" ? 850 : 1050,
      now + duration,
    );
    gain.gain.setValueAtTime(0.0001, now);
    gain.gain.exponentialRampToValueAtTime(0.075, now + 0.035);
    gain.gain.exponentialRampToValueAtTime(0.038, now + 0.16);
    gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);

    source.connect(filter);
    filter.connect(gain);
    gain.connect(context.destination);
    source.addEventListener("ended", () => {
      if (activeSource === source) activeSource = null;
    });
    activeSource = source;
    if (context.state === "suspended") void context.resume();
    source.start(now);
    source.stop(now + duration);
  } catch {
    // Audio is progressive enhancement; face switching must always remain available.
  }
}
