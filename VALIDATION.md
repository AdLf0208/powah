# Validation — Powah slice 1

> **0.2.0 (the SDK package) is recorded in [`notes.md`](notes.md) →
> [`## Migration to the SDK format`](notes.md#migration-to-the-sdk-format)**: its ladder
> (`cargo test` 22 passed, clippy 0 warnings, `fmt --check` clean, `cargo horizon-mod
> build/check/fits`, two byte-identical builds) and its logs (`sdk-*.log`) are there.
> Everything below is the **0.1.0** ladder of the retired `.hnmod`, kept as history: those
> commands (`tools/horizon.py`, `tools/package.py`, `--dev-mod`) no longer exist in this port.

Every check below was run in this session on the campaign pin
**`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`** / mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6), through
`ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`. Logs:
`ports/_shared/evidence/powah/`.

| Check | Command | Result |
|---|---|---|
| format | `cargo fmt --all -- --check` + `rustfmt --check mixins/powah/*.rs` | clean |
| lints | `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| runtime tests | `cargo test --locked --offline` | **14 passed**, 0 failed (written first: 11 failed on the stubs) |
| staged check | `python3 tools/horizon.py check --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, **0 warnings** |
| embedded tests | `python3 tools/horizon.py test …` | `14 passed; 0 failed; 1240 filtered out` |
| fast build | `python3 tools/horizon.py build …` | `cargo_exit: 0` |
| e2e (one delivered file) | `EMBEDDED=1 tools/e2e.sh` on the port fast game, no `--dev-mod` | **PASS**, `EXIT:0` (content from the `.hnmod`) |
| fail-capability | `env EMBEDDED=1 SELFTEST=1 tools/e2e.sh` | exit **1** at the flipped assertion |
| content package | `cargo horizon-mod build` + `check` | 39,262 B, `7d344174…`, 42 entries, content only |
| native package | `tools/package.sh` (content → `data/` → `.hnmod`) | 33,899 B, `0030419c…`, 15 entries incl. the embedded content |
| load-check + negative | `tools/hmod-load.sh` | **PASS**; without `--dev-mod` the ids are unknown |

## Template tools not applicable at this pin

The scaffold ships two fixture tools from the native-mod template. Both are
written for the template's example bundle and its old `ports/horizon` layout, so
they do not validate a real named-hook port here; the real ladder above is the
evidence:

- `python3 tools/test_template.py` → `E_HOOK_CATALOG`: its tiny scratch
  workspace has no `native-hooks.json`, and a real bundle's injections name
  catalog hooks (`block_entity`, …). Its fixture also expects exactly one
  injection and one module.
- `python3 tools/test_stage.py` → `git -C ports/horizon status` fails: this
  campaign's pinned checkout is `ports/_shared/pins/horizon-1d63e0a`.

Both files are kept unmodified so the scaffold stays comparable with
`ports/_template`.

## What the e2e proves, and what it cannot

The run proves the in-game chain (fuel → furnator FE → cable → cell FE), the
controls (no fuel, no cable, full cell) and a cell discharging through a cable,
with a `ctl` transcript in `e2e.log`. It does **not** pin the exact rates: a
chat round trip in this environment is ~2 s (≈40 ticks), far longer than a tick,
so the numeric assertions are bounds and the unit tests carry the exact numbers
(20 FE/t, 30 carbon per burn tick, 48,000 FE per coal, 500/1,000/80 transfers).
