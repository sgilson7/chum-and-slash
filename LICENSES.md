# Licences

Every file in this repository that is not code written for it: its source, its author, its terms. The code is under the repository's licence.

## Assets

| Path | Source | Terms |
| --- | --- | --- |
| `web/art/` | drawn by `tools/art/art.py` from TikZ in this repository | this repository's licence |

## Vendored code

| Path | Project and version | Author | Licence | Checksum |
| --- | --- | --- | --- | --- |

## Code taken from other repositories

| From | What | Licence |
| --- | --- | --- |
| Protection of the Spheres (github.com/sgilson7/pots), Sam Gilson | `crates/core/src/fx.rs`, `geom.rs`, `mesh.rs`, `render.rs`; `web/gl.js`; `data/shaders/scene.vert`, `scene.frag` (stripes added); the shim's pattern; `numbers()` and `uniforms()` | MIT |
| Vagrancy (github.com/sgilson7/vagrancy), Sam Gilson | `crates/core/src/pilot.rs`: `Cond`/`Rule`, the decision loop, node numbering, trace, xorshift64*; the tree data format (`data/pilots.json`) | MIT (on GitHub; the local folder has no LICENSE file) |
| Chumworld Arena (github.com/sgilson7/chumworld-arena), Sam Gilson | the camera knock's ring to zero (`world.rs` `ring`, `RING_PCT`, `QUIET`, `KNOCK_MAX`) and its test sequence | granted under MIT by its author, CS-Q22 (DECISIONS.md), as the repository carries no licence |
| Long Time Comin' (github.com/sgilson7/long-time-comin), Sam Gilson | the skyline rings' placement rule (`scene.rs` `backdrop`, after `arena.rs` `skyline`) | MIT |
| Builder's realtime kit | `crates/core/src/rng.rs`, `ticks.rs`, `sqrt` and `length` in `fx.rs`; the pointer-lock fallback in `web/app.js` | Builder's licence |
| Ape Race (Peter), read only | the idea of a WebGL file that takes every size from the core, already carried by Protection of the Spheres' `gl.js`; nothing copied | no licence: idea credited, not copied |

A file with no licence, a likeness, a recording or any third-party picture is a worklist row in `SECOND-ORDER.md` for the person to decide, and stays out of git until they do.
