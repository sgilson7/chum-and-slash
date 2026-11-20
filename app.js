// The page: it reports raw input to the core, runs the core's frames, draws
// what the core returns, and plays the sounds and rumbles the core lists.
// Which key or button does what, when a prompt shows, where the camera sits:
// all of that is the core's (CLAUDE.md rules 2-4). Pointer lock and the
// `?pointer=free` fallback are after Builder's realtime kit (input.js).
import init, { Game, numbers, mesh, noise, unit } from "./pkg/tool_shim.js?v=ad3bdac955a3";
import { makeRenderer } from "./gl.js?v=ad3bdac955a3";
import { makeAudio } from "./audio.js?v=ad3bdac955a3";

const el = (id) => document.getElementById(id);
const make = (tag, cls, text) => {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
};
const text = async (path) => (await fetch(path)).text();
const json = async (path) => JSON.parse(await text(path));

let game = null;
let renderer = null;
let audio = null;
// T17: the background track, an <audio> element the core turns on and off
let music = null;
let heard = false;
let comboSerial = 0;
let musicTracks = [];
let musicAt = 0;
let N = null;
let Q = 0;
let haptics = {};
let display = {};
let sceneryVersion = -1;
let lastUi = 0;
let engaged = false;
let screen = "";
const free = new URLSearchParams(location.search).get("pointer") === "free";

// ---------------------------------------------------------------- input

function canvasLocked() {
  return document.pointerLockElement === el("view");
}

function engage() {
  engaged = true;
  if (free || !el("view").requestPointerLock) return;
  try {
    const asked = el("view").requestPointerLock({ unadjustedMovement: true });
    if (asked && asked.catch) asked.catch(() => el("view").requestPointerLock());
  } catch (e) {
    engaged = true;
  }
}

function playing() {
  return screen === "fight" || screen === "sandbox" || screen === "cutscene";
}

function listen() {
  const keyEvent = (down) => (e) => {
    if (e.repeat) return;
    if (playing() && (e.code === "Tab" || e.code === "Space")) e.preventDefault();
    game.key(e.code, down);
    if (down && e.code === "Escape" && playing()) {
      game.act("pause", "");
      drawUi(true);
    }
  };
  window.addEventListener("keydown", keyEvent(true));
  window.addEventListener("keyup", keyEvent(false));
  window.addEventListener("blur", () => game.act("pause", ""));
  const view = el("view");
  view.addEventListener("mousedown", (e) => {
    if (!playing()) return;
    if (!canvasLocked() && !free) engage();
    game.key(`Mouse${e.button}`, true);
    e.preventDefault();
  });
  window.addEventListener("mouseup", (e) => game.key(`Mouse${e.button}`, false));
  view.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("mousemove", (e) => {
    if (canvasLocked() || (free && engaged && playing())) game.mouse(e.movementX || 0, e.movementY || 0);
  });
  window.addEventListener("resize", resize);
}

// The first standard-mapped pad, polled each frame (no `gamepadconnected`
// wait, so a scripted pad in the browser check takes the same path).
function pollPad() {
  const pads = navigator.getGamepads ? Array.from(navigator.getGamepads()).filter((p) => p && p.mapping === "standard") : [];
  const [pad] = pads;
  if (!pad) return null;
  const buttons = Uint8Array.from(pad.buttons, (b) => (b.pressed ? 1 : 0));
  game.pad(buttons, Float32Array.from(pad.axes), pad.id);
  return pad;
}

let pad = null;
let lastFocus = "";
let lastStory = "";

function rumble(name) {
  const h = haptics[name];
  const act = pad && pad.vibrationActuator;
  if (!h || !act || !act.playEffect) return;
  act.playEffect("dual-rumble", { startDelay: 0, duration: h.duration, strongMagnitude: h.strong, weakMagnitude: h.weak }).catch(() => {});
}

// ---------------------------------------------------------------- the view

function resize() {
  const view = el("view");
  const dpr = window.devicePixelRatio || 1;
  const scale = (display.slow ? display.software_scale : 1) * (dpr > display.most_dpr ? display.most_dpr : dpr);
  const w = view.clientWidth * scale;
  const h = view.clientHeight * scale;
  renderer.resize(w - (w % 1), h - (h % 1));
  game.act("resize", `${w - (w % 1)}x${h - (h % 1)}`);
}

function meter(label, pct, cls) {
  const box = make("div", `meter ${cls}`);
  box.append(make("span", "meter-label", label));
  const bar = make("span", "meter-bar");
  const fill = make("span", "meter-fill");
  fill.style.width = `${pct}%`;
  bar.append(fill);
  box.append(bar);
  return box;
}

function drawUi(force) {
  const ui = JSON.parse(game.ui());
  const changed = ui.screen !== screen;
  screen = ui.screen;
  document.body.dataset.screen = ui.screen;
  document.body.dataset.device = ui.device;
  if (changed && !playing() && canvasLocked()) document.exitPointerLock();
  const card = el("card");
  const cardShown = ["title", "won", "lost", "paused", "cutscene"].includes(ui.screen);
  card.hidden = !cardShown;
  // T29: a pad moving the focus redraws the card
  const focusKey = JSON.stringify((ui.buttons || []).map((b) => b.focus));
  const refocused = focusKey !== lastFocus;
  lastFocus = focusKey;
  if (cardShown && (changed || force || refocused || ui.screen === "cutscene")) {
    const parts = [];
    if (ui.screen === "title" || ui.screen === "menu") parts.push(make("h1", "game-title", ui.title));
    if (ui.heading) parts.push(make("h2", "heading", ui.heading));
    (ui.lines || []).forEach((l) => parts.push(make("p", "line", l)));
    const buttons = make("div", "buttons");
    (ui.buttons || []).forEach((b) => buttons.append(button(b)));
    if (ui.buttons) parts.push(buttons);
    if (ui.skip) {
      parts.push(make("p", "skip-note", ui.skip.next));
      const skip = meter(ui.skip.label, ui.skip.pct, "skip");
      parts.push(skip);
    }
    card.replaceChildren(...parts);
  }
  if (ui.music) drawMusic(ui.music);
  drawCombo(ui.hud && ui.hud.combo);
  const story = el("story");
  story.hidden = !ui.story;
  // T29: a pad picking a fight redraws the menu
  const storyKey = ui.story ? `${ui.story.brief.title}|${ui.story.pad}` : "";
  const repicked = storyKey !== lastStory;
  lastStory = storyKey;
  if (ui.story && (changed || force || repicked)) {
    drawStory(ui.story, ui.title);
    const picked = document.querySelector('.story-list button[data-picked="true"]');
    if (repicked && picked) picked.scrollIntoView({ block: "nearest" });
  }
  // a pad's first press leaves the title without a click; sound may still
  // wait for one, as browsers ask
  if (changed && audio) audio.unlock();
  // T36: a finisher's black bars; the HUD steps aside while it plays
  document.body.classList.toggle("cinema", Boolean(ui.cinema));
  const finish = el("finish");
  finish.hidden = !(ui.hud && ui.hud.finish);
  finish.textContent = (ui.hud && ui.hud.finish) || "";
  const hud = el("hud");
  hud.hidden = !ui.hud;
  if (ui.hud) {
    const items = [meter(ui.hud.health_label, ui.hud.health, "health"), meter(ui.hud.stamina_label, ui.hud.stamina, "stamina")];
    if (ui.hud.lock) items.push(make("span", "lock", `◎ ${ui.hud.lock_label}`));
    hud.replaceChildren(...items);
  }
  // the style meter keeps its elements, so its fill eases and its pop
  // plays once per rank change
  const style = el("style");
  const s = ui.hud && ui.hud.style;
  style.hidden = !s;
  if (s) {
    style.dataset.rank = s.rank;
    style.classList.toggle("rose", s.rose);
    style.classList.toggle("fell", s.fell);
    style.querySelector(".style-label").textContent = s.label;
    style.querySelector(".style-rank").textContent = s.rank;
    style.querySelector(".style-word").textContent = s.word;
    style.querySelector(".style-fill").style.width = `${s.fill}%`;
  }
  // T12: the locked target's bar keeps its elements, so its fill eases
  const target = el("target");
  const t = ui.hud && ui.hud.target;
  target.hidden = !t;
  if (t) {
    target.querySelector(".target-name").textContent = t.name;
    target.querySelector(".target-fill").style.width = `${t.health}%`;
    const guard = target.querySelector(".target-guard");
    guard.hidden = !t.guarded;
    target.classList.toggle("broken", t.broken);
    target.querySelector(".target-guard-label").textContent = t.guard_label;
    target.querySelector(".target-guard-fill").style.width = `${t.guard}%`;
    // T21: the relaunch timer, shown only while it runs
    const lift = target.querySelector(".target-lift");
    lift.hidden = !(t.lift > 0);
    target.querySelector(".target-lift-label").textContent = t.lift_label;
    target.querySelector(".target-lift-fill").style.width = `${t.lift}%`;
    // T33: what the target's stance reflects
    const stance = target.querySelector(".target-stance");
    stance.hidden = !t.stance;
    stance.textContent = t.stance || "";
  }
  const tools = el("tools");
  tools.hidden = !(ui.screen === "sandbox" && ui.buttons);
  if (!tools.hidden && (changed || force)) {
    tools.replaceChildren(
      ...ui.buttons.map((b) => {
        const button = make("button", "", b.label);
        button.dataset.act = b.act;
        button.dataset.value = b.value;
        button.addEventListener("click", () => press(b.act, b.value));
        return button;
      }),
    );
  }
  const prompt = el("prompt");
  prompt.hidden = !ui.prompt;
  document.body.dataset.prompt = ui.prompt ? ui.prompt.action : "";
  if (ui.prompt) {
    const kids = [make("span", "prompt-button", ui.prompt.button), make("span", "prompt-label", ui.prompt.label)];
    if (ui.prompt.hint) kids.push(make("span", "prompt-hint", ui.prompt.hint));
    prompt.classList.toggle("paused", ui.prompt.paused);
    prompt.replaceChildren(...kids);
  }
  const strip = el("strip");
  strip.hidden = !ui.strip;
  if (ui.strip) {
    const items = ui.strip.items.map((s) => {
      const item = make("span", `step${s.done ? " done" : ""}${s.next ? " next" : ""}`);
      item.append(make("b", "", s.button), make("span", "", s.label));
      return item;
    });
    if (ui.strip.note) items.push(make("span", "strip-note", ui.strip.note));
    strip.replaceChildren(...items);
  }
}

// T14: the story menu, after Choomstar's missions screen: the route map,
// the fights by chapter, and the picked fight's briefing. Every place,
// number and word comes from the core's `story`.
const SVG = "http://www.w3.org/2000/svg";
const svg = (tag, attrs, text) => {
  const node = document.createElementNS(SVG, tag);
  Object.entries(attrs).forEach(([k, v]) => node.setAttribute(k, v));
  if (text !== undefined) node.textContent = text;
  return node;
};
const button = (b) => {
  const node = make("button", b.main ? "main" : "", b.label);
  node.dataset.act = b.act;
  node.dataset.value = b.value;
  // T29: the button a pad has focused, and the pad button that presses it
  if (b.focus) {
    node.classList.add("focused");
    if (b.pad) node.append(make("span", "pad-key", b.pad));
  }
  node.addEventListener("click", () => press(b.act, b.value));
  return node;
};

function drawStory(s, title) {
  const head = make("header", "story-head");
  head.append(make("h1", "game-title", title), make("p", "story-tag", s.tag));
  if (s.pad) head.append(make("p", "story-pad", s.pad));
  const actions = make("div", "story-actions");
  s.actions.forEach((b) => actions.append(button(b)));
  head.append(actions);

  const map = make("div", "story-panel story-map");
  // T33: one tab a world; the map and the list show the picked one
  if (s.worlds && s.worlds.length > 1) {
    const tabs = make("div", "story-worlds");
    s.worlds.forEach((w) => {
      const t = make("button", "world-tab", w.label);
      t.dataset.act = w.act;
      t.dataset.value = w.value;
      t.dataset.picked = w.picked;
      t.addEventListener("click", () => press(w.act, w.value));
      tabs.append(t);
    });
    map.append(tabs);
  }
  const m = s.map;
  const chart = svg("svg", { viewBox: m.view, role: "img", "aria-label": m.label });
  chart.append(svg("path", { class: "road", d: m.road }), svg("path", { class: "road-lit", d: m.lit }));
  m.bands.forEach((b) => chart.append(svg("text", { class: "band", x: b.x, y: b.y }, b.label)));
  m.nodes.forEach((n) => {
    const g = svg("g", { class: n.small ? "node small" : "node", "data-picked": n.picked, "data-done": n.done, tabindex: "-1" });
    g.append(svg("circle", { class: "dot", cx: n.x, cy: n.y, r: n.r || m.r }));
    g.append(svg("text", { class: "node-number", x: n.x, y: n.y }, n.number));
    g.append(svg("text", { class: "node-name", x: n.x, y: n.name_y }, n.name));
    if (n.mark) g.append(svg("text", { class: "node-mark", x: n.mark_x, y: n.y }, n.mark));
    g.addEventListener("click", () => press("pick", n.value));
    chart.append(g);
  });
  map.append(chart);

  const list = make("div", "story-panel story-list");
  s.chapters.forEach((c) => {
    list.append(make("h2", "", c.label));
    const ol = make("ol");
    c.entries.forEach((e) => {
      const b = make("button", "entry");
      b.dataset.picked = e.picked;
      b.append(make("span", "number", e.number), make("span", "name", e.name), make("span", "kind", e.kind), make("span", "mark", e.mark));
      b.addEventListener("click", () => press("pick", e.value));
      b.addEventListener("dblclick", () => press("arena", e.value));
      const li = make("li");
      li.append(b);
      ol.append(li);
    });
    list.append(ol);
  });

  const r = s.brief;
  const brief = make("div", "story-panel story-brief");
  brief.append(make("p", "chapter", r.chapter), make("h3", "", r.title));
  r.lines.forEach((l) => brief.append(make("p", "", l)));
  brief.append(make("h4", "", r.foes_label));
  const foes = make("ul", "foes");
  r.foes.forEach((f) => {
    const li = make("li");
    li.append(make("b", "", f.count), document.createTextNode(f.name));
    foes.append(li);
  });
  brief.append(foes);
  if (r.best) brief.append(make("p", "best", r.best));
  const go = make("div", "buttons");
  r.buttons.forEach((b) => go.append(button(b)));
  brief.append(go, make("h4", "", r.moves_label));
  const table = make("table", "moves");
  r.moves.forEach(([keys, name]) => {
    const tr = make("tr");
    tr.append(make("td", "", keys), make("td", "", name));
    table.append(tr);
  });
  brief.append(table, make("p", "", s.controls));

  el("story").replaceChildren(head, map, list, brief);
}

// T17: the music control (on or off, and a volume) on the title, the menu
// and the pause card; the core holds both settings and says where it shows
function drawMusic(m) {
  const box = el("music");
  box.hidden = !m.shown;
  if (!music) {
    // T37: the tracks in turn, starting over after the last
    music = musicTracks.length ? new Audio(musicTracks[0]) : { paused: true, play: () => Promise.resolve(), pause: () => {} };
    if (musicTracks.length) {
      music.loop = musicTracks.length === 1;
      music.addEventListener("ended", () => {
        musicAt = (musicAt + 1) % musicTracks.length;
        music.src = musicTracks[musicAt];
        music.play().catch(() => {});
      });
    }
    box.querySelector(".music-toggle").addEventListener("click", () => press("music", ""));
    box.querySelector(".music-volume").addEventListener("input", (e) => press("volume", e.target.value));
    const missing = box.querySelector(".music-missing");
    missing.textContent = m.missing;
    missing.hidden = musicTracks.length > 0;
  }
  box.querySelector(".music-label").textContent = m.label;
  const toggle = box.querySelector(".music-toggle");
  toggle.textContent = m.on ? m.on_label : m.off_label;
  toggle.setAttribute("aria-pressed", String(m.on));
  const slider = box.querySelector(".music-volume");
  slider.max = m.max;
  slider.setAttribute("aria-label", m.volume_label);
  if (document.activeElement !== slider) slider.value = m.volume;
  music.volume = m.volume / m.max;
  if (m.on && heard && music.paused) music.play().catch(() => {});
  if (!m.on && !music.paused) music.pause();
}

// T18: a named combo's name across the top, restarted for each new one
function drawCombo(c) {
  const box = el("combo");
  box.hidden = !c;
  if (!c || c.serial === comboSerial) return;
  comboSerial = c.serial;
  box.querySelector(".combo-name").textContent = c.name;
  box.classList.remove("pop");
  void box.offsetWidth;
  box.classList.add("pop");
}

function press(act, value) {
  heard = true;
  if (act === "start") audio.unlock();
  const res = JSON.parse(game.act(act, value));
  if (res.error) el("message").textContent = res.error;
  drawUi(true);
  if (playing()) engage();
}

// ---------------------------------------------------------------- the loop

function loop(now) {
  pad = pollPad();
  game.frame(now);
  for (const e of JSON.parse(game.events())) {
    if (e.kind === "sound") audio.play(e.name, e.rate * Q, e.gain * Q);
    if (e.kind === "rumble") rumble(e.name);
    if (e.kind === "screen") drawUi(true);
  }
  // T36: the finisher's flash and bars, every frame
  const [flash, bars] = game.cinema();
  el("flash").style.opacity = flash;
  el("bars").style.setProperty("--bars", bars);
  game.render();
  const v = game.scenery_version();
  if (v !== sceneryVersion) {
    sceneryVersion = v;
    renderer.scenery(game.scenery());
  }
  renderer.setUniforms(game.uniforms(), game.clear_color());
  renderer.frame(game.mats(), game.draws(), game.scene());
  if (now - lastUi >= N.ui_every_ms) {
    lastUi = now;
    drawUi(false);
    document.body.dataset.probe = game.probe();
  }
  requestAnimationFrame(loop);
}

async function main() {
  await init();
  N = JSON.parse(numbers());
  Q = unit();
  const [vertex, fragment, recipes, h, d, words, tune] = await Promise.all([
    text("data/shaders/scene.vert"),
    text("data/shaders/scene.frag"),
    json("data/audio.json"),
    json("data/haptics.json"),
    json("data/display.json"),
    json("data/text.json"),
    json("data/music.json"),
  ]);
  musicTracks = tune.tracks || [];
  haptics = h;
  display = d;
  game = new Game();
  renderer = makeRenderer(el("view"), N, { vertex, fragment }, (id) => mesh(id));
  if (!renderer) {
    el("message").textContent = words.no_webgl;
    document.body.dataset.ready = "true";
    return;
  }
  display.slow = renderer.software(display.software);
  // T16: the core leaves the grass out for a software renderer
  // (`?detail=full` keeps it, for the journal's pictures)
  if (display.slow && new URLSearchParams(location.search).get("detail") !== "full") game.act("software", "1");
  audio = makeAudio(recipes, noise(N.noise_samples));
  listen();
  resize();
  drawUi(true);
  document.body.dataset.ready = "true";
  requestAnimationFrame(loop);
}

main();
