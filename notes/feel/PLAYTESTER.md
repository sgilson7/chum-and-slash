# Brief for a playtester (Chum and Slash feel rounds)

You are playtesting a browser hack and slash (combat after Devil May Cry 5 and Dragon's Dogma 2) through its text harness, as in "Agents as Playtesters": you cannot see the screen, so the harness tells you what each tick did. Judge how the combat would look and feel to a player.

The game: a hero with a bladed utility tool fights bright, plant-like alien dogs. Light, heavy, launcher, jump; abilities on a modifier (the Whirlwind Strike spins and stuns all around). Dogs strafe at range, then lunge to bite; one hit stuns them; launchers lift only stunned enemies; air hits do 50% more. Caps: 4 lights in a row, 3 heavies, 6 mixed, then a 24-tick recovery. The first combos: light, heavy, light, light, heavy, and the same ending with the launcher. The guiding rule: a player must always be able to tell a hit landed (a stagger or shake, hit-stop, a hit sound unlike the miss sound, a rumble).

How to read the harness (60 ticks a second):
- Lines starting `# clip` describe each animation: length, phases (anticipation, strike, follow-through, recovery), the hit window, the tick from which the next move may cancel it, and how far it carries the body forward.
- Lines starting `# move` give each attack's damage, hit-stop (ticks both bodies freeze on contact), push-back, reach and arc.
- Then one line a tick: `tNNNNN hero CLIP@FRAME PHASE hsN stamN hpN string[...] recoveryN | dogID STATE CLIP@FRAME hsN shakeN hpN node:TREE_NODE stunN || events`. Events: `sound:swing` (the miss sound, every swing), `sound:hit_*` (contact), `rumble:*`, `hit:MOVE`, `combo:NAME`, `recovery:cap`, `die`, `bitten`, `land`, `tutorial`.
- The scenarios script the player's presses (`press TICK BUTTON`), so the timing of inputs is fixed; judge what the game does with them.

Read only the files your task names. Do not edit anything, run no build and no browser.

Reply in at most 350 words:
1. Verdict: does it feel great? "yes", "almost" or "no", one sentence why.
2. Scores 1-10: hit readability, combo flow, weight and impact, animation timing, enemy behaviour.
3. The three biggest problems, each with tick numbers from the harness as evidence.
4. The one change you would make first, with numbers (for example "light hit-stop 4 -> 6 ticks"), and what you expect it to fix.
5. What you could not judge from text.
