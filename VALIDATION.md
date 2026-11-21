# Validation — Powah slices 1 to 5

> **0.2.5 (slice 5, the Energizing Orb, its seven rods and the materials they make)** is recorded in
> [`notes.md`](notes.md#slice-5--the-energizing-orb-the-rod-ladder-and-the-materials-2026-11-21):
> the slice and its gap line (slice 4's five `blocked(energizing-orb)` cells), the red proof
> (seven `todo!()` tests, 34 passed / 7 failed, plus a second red for the orb's own rule at
> 39 passed / 3 failed), the ladder at pin `0a875da` (42 tests, clippy 0 warnings, `fmt --check`
> clean, two byte-identical builds of 210,786 B, `check`, `fits` against the pin **and** against
> `0b14f8a`, gate 1/1 with **292 entries**) and the in-game legs (main `PASS`+`EXIT:0`, control /
> `SELFTEST=1` / `SELFTEST=orb` at `EXIT:1`, `check-e2e-logs --strict` 12/12). Package:
> `powah.hmod`, 210,853 B, sha256 `7aef65e337192bb0caff50946ceb889e5f40bfcc10892654e766d46d78e697c3`.
> **0.2.4 (slice 4, the energy cell ladder)** is recorded in
> [`notes.md`](notes.md#slice-4--the-energy-cell-ladder-2026-11-21): the slice and its gap line, the red
> proof (three `todo!()` tests, 28 passed / 3 failed, plus the 0.2.3 package failing at the first new
> tier), the ladder at pin `0a875da` (32 tests, clippy 0 warnings, `fmt --check` clean, two
> byte-identical builds of 119,200 B, `check`, `fits` against the pin **and** against `0b14f8a`, gate
> 1/1 with 134 entries) and the in-game legs (main `PASS`+`EXIT:0`, control / `SELFTEST=1` /
> `SELFTEST=cells` at `EXIT:1`). Package: `powah.hmod`, 119,200 B, sha256
> `663465ee8b93e5f8be371537f6515bd7252651e462a57f477faedd82ffcd3e34`.
> **0.2.3 (slice 3, the cable network)** is recorded in [`notes.md`](notes.md#slice-3--the-cable-network-2026-11-21):
> the slice and its gap line, the red proof (five `todo!()` tests, 23 passed / 5 failed), the
> ladder at pin `0a875da` (28 tests, clippy 0 warnings, `fmt --check` clean, two byte-identical
> builds of 85,383 B, `check`, `fits` against the pin **and** against `0b14f8a`, gate 1/1) and the
> in-game legs (main `PASS`+`EXIT:0`, control / `SELFTEST=network` / `SELFTEST=1` at `EXIT:1`,
> the 0.2.2 build red at `EXIT:1`, and `check-e2e-logs.py --strict` 9/9).
> **0.2.2 (slice 2, the dielectric/capacitor family) and 0.2.1 (the review fixes) are recorded in
> [`notes.md`](notes.md)**: [`## Slice 2 …`](notes.md#slice-2--the-dielectriccapacitor-crafting-family-2026-11-20)
> carries its ladder, its in-game legs and the coverage table, and
> [`## Verification (0.2.1)`](notes.md#verification-021) the review-fix run.
> **0.2.0 (the SDK package)** is under
> [`## Migration to the SDK format`](notes.md#migration-to-the-sdk-format): its ladder
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
