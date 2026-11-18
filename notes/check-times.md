# Check times

One row per run of ./scripts/quick-check.sh, newest last. A quick check skips relative to the newest passing `check` row.

| When | Commit | Check | Steps (s) | Total (s) | Result |
| --- | --- | --- | --- | --- | --- |
| 2026-11-18 16:54 | 97c50a4+ | quick | fmt=0.1 clippy=1.1 core=5.9 build=2.5 wasm=0.3 notebook=0.0 | 10.0 | FAIL at notebook |
| 2026-11-18 16:54 | 97c50a4+ | quick | fmt=0.1 clippy=0.1 core=0.4 build=0.2 wasm=0.3 notebook=0.1 context=0.2 journal=0.1 | 1.5 | FAIL at journal |
| 2026-11-18 17:15 | 903b107+ | quick | fmt=0.1 clippy=1.5 core=6.5 build=2.7 wasm=0.3 notebook=0.1 context=0.2 journal=0.1 browser=44.3 | 55.9 | pass |
