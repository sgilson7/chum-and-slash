"""Chum and Slash's browser interactions, run by check.py in every engine.

check.py has loaded the page, waited for it to report ready, and let it idle.
These drive the game as a person would: start, open the sandbox, spawn a dog,
fight it with the keyboard and mouse, then the same with a scripted
standard-mapped gamepad (CS-14), and play into the first arena far enough to
see its cutscene skip and its fight begin (CS-13). They wait on selectors and
the data-* attributes the page sets, never on a fixed time (D7).

Expected values come from the brief, not from the game's own output: the
title is "Chum and Slash"; a pad's Square (index 2) is the light attack and
its hits rumble (cue 104).
"""

import json

# A standard-mapped DualSense the page polls each frame, and a record of the
# rumbles it is asked for. The page takes the same path as for a real pad:
# it never listens for `gamepadconnected`, so nothing here is a test-only
# branch.
FAKE_PAD = """
(() => {
  const buttons = Array.from({length: 17}, () => ({pressed: false, value: 0}));
  window.__rumbles = [];
  window.__pad = {
    id: 'DualSense Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 0ce6)',
    index: 0, connected: true, mapping: 'standard', timestamp: 0,
    axes: [0, 0, 0, 0], buttons,
    vibrationActuator: {type: 'dual-rumble', playEffect: (kind, p) => { window.__rumbles.push(kind); return Promise.resolve('complete'); }},
  };
  window.__press = (i, on) => { buttons[i].pressed = on; buttons[i].value = on ? 1 : 0; };
  // the left stick pushed forward walks the hero toward the locked dog
  navigator.getGamepads = () => [window.__pad];
})();
"""


def probe(page):
    return json.loads(page.get_attribute("body", "data-probe") or "{}")


def wait_screen(page, name):
    page.wait_for_selector(f"body[data-screen='{name}']", state="attached", timeout=30000)


def wait_probe(page, key, test, what, check, tries=600):
    for _ in range(tries):
        p = probe(page)
        if key in p and test(p[key]):
            check(True, what)
            return p
        page.evaluate("() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
    check(False, f"{what} (last probe {probe(page)})")
    return probe(page)


def run(page, check, engine):
    base = page.url.split("?")[0]
    page.goto(base + "?pointer=free")
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    check(page.inner_text(".game-title") == "Chum and Slash", "the title is drawn")
    check(page.get_attribute("body", "data-screen") == "title", "the game opens on its title screen")
    check(page.inner_text("#message") == "", "no error message is shown")

    page.click("button[data-act='start']")
    wait_screen(page, "menu")
    check(page.is_visible("button[data-act='prologue']"), "the menu offers the prologue")
    page.click("button[data-act='sandbox']")
    wait_screen(page, "sandbox")
    check(page.is_visible("#hud"), "the sandbox shows the health and stamina gauges")

    # spawn a dog and fight it with the keyboard and mouse
    page.click("button[data-act='spawn'][data-value='dog']")
    wait_probe(page, "enemies", lambda n: n == 1, "the sandbox spawns a dog", check)
    view = page.locator("#view")
    for _ in range(4):
        view.dispatch_event("mousedown", {"button": 0})
        page.dispatch_event("body", "mouseup", {"button": 0})
        page.keyboard.down("KeyW")
        page.evaluate("() => new Promise(r => setTimeout(r, 120))")
        page.keyboard.up("KeyW")
    wait_probe(page, "tick", lambda t: t > 30, "the fight runs on its tick", check)

    # T3: Escape pauses, and the pause card spawns without resuming
    page.keyboard.press("Escape")
    wait_screen(page, "paused")
    page.click("#card button[data-act='spawn'][data-value='pup']")
    wait_probe(page, "enemies", lambda n: n == 2, "the sandbox pause card spawns a dog", check)
    check(page.get_attribute("body", "data-screen") == "paused", "spawning from the pause card keeps the game paused")
    page.click("#card button[data-act='resume']")
    wait_screen(page, "sandbox")

    # a scripted pad: Square is the light attack, and a landed hit rumbles
    page.add_init_script(FAKE_PAD)
    page.goto(base + "?pointer=free")
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    page.click("button[data-act='start']")
    page.click("button[data-act='sandbox']")
    wait_screen(page, "sandbox")
    page.click("button[data-act='spawn'][data-value='dog']")
    wait_probe(page, "device", lambda d: d in ("keyboard", "playstation"), "the page polls the pad", check)
    landed = False
    for _ in range(80):
        page.evaluate("() => { window.__pad.axes[1] = -1; }")
        page.evaluate("() => window.__press(2, true)")
        page.evaluate("() => new Promise(r => setTimeout(r, 60))")
        page.evaluate("() => window.__press(2, false)")
        page.evaluate("() => new Promise(r => setTimeout(r, 120))")
        if page.evaluate("() => window.__rumbles.length") > 0:
            landed = True
            break
    page.evaluate("() => { window.__pad.axes[1] = 0; }")
    check(probe(page).get("device") == "playstation", "the pad is known as a PlayStation pad")
    check(landed, "a light hit from the pad's Square rumbles the pad")

    # the prologue: its cutscene skips, and its fight begins
    page.goto(base + "?pointer=free")
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    page.click("button[data-act='start']")
    page.click("button[data-act='prologue']")
    wait_screen(page, "cutscene")
    check(page.inner_text(".heading") != "", "the cutscene names its arena")
    page.keyboard.down("KeyX")
    wait_screen(page, "fight")
    page.keyboard.up("KeyX")
    check(probe(page).get("alive") == 1, "Arena 1 has one dog")
    check(probe(page).get("instances", 0) > 100, "the backdrop and the bodies are drawn")
