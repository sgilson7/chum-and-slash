# State

The project's long-term memory, at most two pages. Rewrite it at each milestone (a stage boundary, a finished feature, a deploy) and before a handoff. After a compaction or in a new session, read this file and ask `tools/ask.py` for cards; do not re-read files it already summarises. `scripts/context_check.py` checks its size, its commit and the cards it cites.

Commit: 4c03098

## Where the build is

Stage 2 done, stopped at stage 3: Sam reads `PLAN.md` and answers CS-Q1 to CS-Q25. No game code yet. Brief holds the transcript word for word with readings. Journal (`../hack-and-slash-journal/`, private artifact https://claude.ai/artifact/HFNKrsPgkiULwcmrmEADUx) has the opening entry, the brief, seven research entries and the plan. Digests: `../refs/pots.md` and `../refs/pursuer.md` written; `../refs/vagrancy.md` gained a Behaviour trees bullet. Shallow clones in `../refs-src/`.

## Decisions in force

- Rules CS-1 to CS-20 (journal opening entry); every question on its default until answered.
- CS-Q12 default: Q16.16 core (D2). CS-Q11 default: port POTS's renderer.
- D21 changed: subagents named in BRIEF.md (reviewers done; playtesters per CS-Q15).
- Deploy grant: public repo Chum and Slash (`chum-and-slash`, CS-Q10) when `check.sh` passes.
- Person rows: R1.3 (Chumworld Arena licence, CS-Q22), R1.4 (install Playwright).

## What is next

Read Sam's answers from the journal's `answers` store (ArtifactData list), record with `decisions.py --answers`, write an *agreed* entry, move lasting rules into CLAUDE.md, then build M0 (realtime kit, `World::tick`, harness).

## Cards the next session needs

`card:docs.context.how-to-read-builder`
`card:docs.journal.answer-chips`
`card:kits.realtime.drawing-less-on-a-software-renderer`
`card:kits.canvas.going-further-webgl`
