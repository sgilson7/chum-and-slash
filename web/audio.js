// Sounds, built from the recipes in data/audio.json with Web Audio. The core
// decides which sound plays, at what pitch and gain (a hit unlike a miss,
// a combo rising a semitone a hit; CS-2); this file only builds the layers.
// The noise comes from the core too, so the page holds no randomness and no
// numbers of its own (D3). Audio starts only after a click (every browser
// holds it until then), so the page calls unlock() from its start button.

export function makeAudio(recipes, noiseSamples) {
  let ctx = null;
  let out = null;
  let noise = null;

  function unlock() {
    if (ctx) return ctx.resume();
    const Ctor = window.AudioContext || window.webkitAudioContext;
    if (!Ctor) return Promise.resolve();
    ctx = new Ctor({ latencyHint: "interactive" });
    out = ctx.createGain();
    out.gain.value = recipes.master;
    out.connect(ctx.destination);
    noise = ctx.createBuffer(1, noiseSamples.length, ctx.sampleRate);
    noise.copyToChannel(noiseSamples, 0);
    return ctx.resume();
  }

  function layer(l, at, rate, gain) {
    const env = ctx.createGain();
    env.gain.setValueAtTime(0, at);
    env.gain.linearRampToValueAtTime(l.gain * gain, at + l.attack);
    env.gain.linearRampToValueAtTime(0, at + l.dur);
    let src;
    if (l.src === "noise") {
      src = ctx.createBufferSource();
      src.buffer = noise;
      src.playbackRate.value = rate;
      const f = ctx.createBiquadFilter();
      f.type = l.filter;
      f.Q.value = l.q;
      f.frequency.setValueAtTime(l.f0 * rate, at);
      f.frequency.linearRampToValueAtTime(l.f1 * rate, at + l.dur);
      src.connect(f);
      f.connect(env);
    } else {
      src = ctx.createOscillator();
      src.type = l.src;
      src.frequency.setValueAtTime(l.f0 * rate, at);
      src.frequency.linearRampToValueAtTime(l.f1 * rate, at + l.dur);
      src.connect(env);
    }
    env.connect(out);
    src.start(at);
    src.stop(at + l.dur);
  }

  return {
    unlock,
    play(name, rate, gain) {
      const layers = recipes.sounds[name];
      if (!ctx || !layers || ctx.state !== "running") return;
      const at = ctx.currentTime;
      for (const l of layers) layer(l, at, rate, gain);
    },
  };
}
