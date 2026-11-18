#!/usr/bin/env python3
"""Screenshots of the running build for the journal (ENTRIES.md, Pictures).

Serves dist/ on a local port, drives the game headless in Chromium with
?pointer=free, and saves PNGs under the journal's img/build/.

    .venv/bin/python tools/shots.py
"""
import functools
import http.server
import pathlib
import socketserver
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT.parent / "hack-and-slash-journal" / "img" / "build"


class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


def serve():
    handler = functools.partial(Quiet, directory=str(ROOT / "dist"))
    httpd = socketserver.TCPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return httpd, f"http://127.0.0.1:{httpd.server_address[1]}"


def frames(page, n):
    for _ in range(n):
        page.evaluate("() => new Promise(r => requestAnimationFrame(r))")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    httpd, base = serve()
    with sync_playwright() as p:
        b = p.chromium.launch(args=["--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        page = b.new_page(viewport={"width": 1280, "height": 720})
        page.goto(base + "/index.html?pointer=free")
        page.wait_for_selector("body[data-ready='true']", state="attached")
        frames(page, 30)
        page.screenshot(path=str(OUT / "title.png"))
        page.click("button[data-act='start']")
        page.click("button[data-act='sandbox']")
        page.click("button[data-act='spawn'][data-value='dog']")
        page.click("button[data-act='spawn'][data-value='big_dog']")
        frames(page, 40)
        page.screenshot(path=str(OUT / "sandbox.png"))
        page.locator("#view").dispatch_event("mousedown", {"button": 0})
        page.dispatch_event("body", "mouseup", {"button": 0})
        frames(page, 10)
        page.screenshot(path=str(OUT / "swing.png"))
        page.goto(base + "/index.html?pointer=free")
        page.wait_for_selector("body[data-ready='true']", state="attached")
        page.click("button[data-act='start']")
        page.click("button[data-act='arena'][data-value='0']")
        frames(page, 30)
        page.screenshot(path=str(OUT / "arena1-cutscene.png"))
        page.keyboard.down("KeyX")
        page.wait_for_selector("body[data-screen='fight']", state="attached")
        page.keyboard.up("KeyX")
        frames(page, 30)
        page.screenshot(path=str(OUT / "arena1-fight.png"))
        # Arena 4 (T4): four dogs and a large dog, a second into the fight
        page.goto(base + "/index.html?pointer=free")
        page.wait_for_selector("body[data-ready='true']", state="attached")
        page.click("button[data-act='start']")
        page.click("button[data-act='arena'][data-value='3']")
        frames(page, 30)
        page.screenshot(path=str(OUT / "arena4-cutscene.png"))
        page.keyboard.down("KeyX")
        page.wait_for_selector("body[data-screen='fight']", state="attached")
        page.keyboard.up("KeyX")
        frames(page, 60)
        page.screenshot(path=str(OUT / "arena4-fight.png"))
        b.close()
    httpd.shutdown()
    print("wrote", sorted(x.name for x in OUT.glob("*.png")))


if __name__ == "__main__":
    main()
