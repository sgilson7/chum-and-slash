# Check times

One row per run of ./scripts/quick-check.sh, newest last. A quick check skips relative to the newest passing `check` row.

| When | Commit | Check | Steps (s) | Total (s) | Result |
| --- | --- | --- | --- | --- | --- |
| 2026-11-18 16:54 | 97c50a4+ | quick | fmt=0.1 clippy=1.1 core=5.9 build=2.5 wasm=0.3 notebook=0.0 | 10.0 | FAIL at notebook |
| 2026-11-18 16:54 | 97c50a4+ | quick | fmt=0.1 clippy=0.1 core=0.4 build=0.2 wasm=0.3 notebook=0.1 context=0.2 journal=0.1 | 1.5 | FAIL at journal |
| 2026-11-18 17:15 | 903b107+ | quick | fmt=0.1 clippy=1.5 core=6.5 build=2.7 wasm=0.3 notebook=0.1 context=0.2 journal=0.1 browser=44.3 | 55.9 | pass |
| 2026-11-18 17:27 | 7e1c206+ | quick | fmt=0.1 clippy=1.0 | 1.2 | FAIL at clippy |
| 2026-11-18 17:28 | 7e1c206+ | quick | fmt=0.1 clippy=1.7 core=10.3 build=2.6 wasm=0.3 browser=43.0 | 58.0 | pass |
| 2026-11-18 22:31 | debbc3f+ | quick | fmt=0.1 clippy=2.4 | 2.6 | FAIL at clippy |
| 2026-11-18 22:32 | debbc3f+ | quick | fmt=0.1 clippy=2.0 core=11.9 build=2.6 wasm=0.4 browser=44.1 | 61.1 | pass |
| 2026-11-18 22:41 | debbc3f+ | quick | fmt=0.1 clippy=2.3 core=12.4 build=2.6 wasm=0.4 browser=42.8 | 60.6 | pass |
