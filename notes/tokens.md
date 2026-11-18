# Token use

One row per measured session, newest last (`python3 tools/tokens.py`). Each call re-reads the whole context, so re-read ≈ mean context × calls.

| When | Session | Calls | Context at first plan | Peak | Mean | Compactions | Re-read | Above 200k |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-18 14:35 | 07850e6c | 254 | 155k (call 31) | 659k | 398k | 1 | 100.3M | 53% |
