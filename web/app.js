// The page: it reports raw input to the core, runs the core's frames, draws
// what the core returns, and plays the sounds and rumbles the core lists.
// Which key or button does what, when a prompt shows, where the camera sits:
// all of that is the core's (CLAUDE.md rules 2-4). Pointer lock and the
// `?pointer=free` fallback are after Builder's realtime kit (input.js).
import init, { Game, numbers, mesh, noise, unit } from "./pkg/tool_shim.js";
import { makeRenderer } from "./gl.js";
import { makeAudio } from "./audio.js";

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
  const cardShown = ["title", "menu", "won", "lost", "paused", "cutscene"].includes(ui.screen);
  card.hidden = !cardShown;
  if (cardShown && (changed || force || ui.screen === "cutscene")) {
    const parts = [];
    if (ui.screen === "title" || ui.screen === "menu") parts.push(make("h1", "game-title", ui.title));
    if (ui.heading) parts.push(make("h2", "heading", ui.heading));
    (ui.lines || []).forEach((l) => parts.push(make("p", "line", l)));
    const buttons = make("div", "buttons");
    (ui.buttons || []).forEach((b) => {
      const button = make("button", "", b.label);
      button.dataset.act = b.act;
      button.dataset.value = b.value;
      button.addEventListener("click", () => press(b.act, b.value));
      buttons.append(button);
    });
    if (ui.buttons) parts.push(buttons);
    if (ui.skip) {
      parts.push(make("p", "skip-note", ui.skip.next));
      const skip = meter(ui.skip.label, ui.skip.pct, "skip");
      parts.push(skip);
    }
    card.replaceChildren(...parts);
  }
  const hud = el("hud");
  hud.hidden = !ui.hud;
  if (ui.hud) {
    const items = [meter(ui.hud.health_label, ui.hud.health, "health"), meter(ui.hud.stamina_label, ui.hud.stamina, "stamina")];
    if (ui.hud.lock) items.push(make("span", "lock", `◎ ${ui.hud.lock_label}`));
    if (ui.hud.style) items.push(make("span", "style", `${ui.hud.style_label} ${ui.hud.style}`));
    hud.replaceChildren(...items);
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

function press(act, value) {
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
  const [vertex, fragment, recipes, h, d, words] = await Promise.all([
    text("data/shaders/scene.vert"),
    text("data/shaders/scene.frag"),
    json("data/audio.json"),
    json("data/haptics.json"),
    json("data/display.json"),
    json("data/text.json"),
  ]);
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
  audio = makeAudio(recipes, noise(N.noise_samples));
  listen();
  resize();
  drawUi(true);
  document.body.dataset.ready = "true";
  requestAnimationFrame(loop);
}

main();
