# Powah! (Rearchitected) — a Horizon port

**Powah! (Rearchitected)** is a Minecraft mod. Discussion on the AE2 Discord server:. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Technici4n/Powah](https://github.com/Technici4n/Powah) — licence **LGPL-3.0-only**.

| | |
|---|---|
| Upstream mod | Powah! (Rearchitected) ([source](https://github.com/Technici4n/Powah)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `powah.hmod` (install it with the Horizon launcher) |
| Licence | **LGPL-3.0-only** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **Furnator** (`powah:furnator_starter`): burns furnace fuel into a carbon
- **Energy Cell** (`powah:energy_cell_starter`): a 1,000,000 FE capacitor that
- **Energy Cable** (`powah:energy_cable_starter`): moves FE between the block

## Not yet ported

- **No GUI.** Upstream opens a container screen for the furnator/cell/cable;
- **No cable network.** Upstream's cables share one buffer across a connected
- **Two adjacent cells swap energy.** Both push on every side by default (as
- **No item charging.** Upstream's furnator charges slot 0 and a cell charges
- **Breaking a machine loses its FE.** Upstream's `keepEnergy()` makes the
- **No side config or redstone modes.** Upstream's six per-side transfer types
- **No comparator output.** `comparator_strength` is computed and printed, but
- **`lit`/`burning` is not a block state** (upstream's `burning` is a client

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `powah.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Powah! (Rearchitected) was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **LGPL-3.0-only**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Technici4n/Powah

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run powah` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/powah/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
