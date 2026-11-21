# Powah for Horizon — slices 1–3

A clean-room Rust rewrite of **Powah! (Rearchitected) 6.2.10** (`Technici4n/Powah`,
**LGPL-3.0-only**) for Horizon 26.3, as one flat SDK package (`api = "0.2"`): the
carried content (`assets/`, `data/`) and the behaviour (`src/`) ship in the same
`.hmod`. Slice 1 is the starter tier of the energy chain:

| Block | What it does |
|---|---|
| `powah:furnator_starter` | burns furnace fuel into a carbon buffer and makes 20 FE/t into a 20,000 FE buffer (48,000 FE per coal) |
| `powah:energy_cell_starter` | a 1,000,000 FE capacitor, 1,000 FE/t per side |
| `powah:energy_cable_starter` | moves FE between the block entities beside it, 500 FE/t, storing nothing |

Slice 2 is the **dielectric/capacitor crafting family** — `powah:dielectric_paste`,
`dielectric_rod`, `dielectric_rod_horizontal`, `dielectric_casing`, `capacitor_basic`
and `capacitor_basic_tiny`, with their eight recipes — which is what the three
machine recipes above need, so all three now craft.

Slice 3 is the **cable network**: every cable connected to a cable through cables is
one relay (upstream's `CableNet`), so a charge crosses a run of three cables into a
cell at the far end (0.2.2 moved nothing across more than one hop). The network
carries 500 FE/t — one cable's rate for the whole run — and creates or loses no FE
across it.

All three declare a real `Capabilities.Energy.BLOCK` buffer (the amount is the int
`energy` in the block entity's saved compound), so other machines can fill and
drain the furnator and the cell; the cable is a pure relay.

- Scope, behaviour table, the hook table, deviations and the gap list:
  [`notes.md`](notes.md); upstream analysis: [`FINDINGS.md`](FINDINGS.md);
  content carry and the patches: [`CONTENT.md`](CONTENT.md).
- Verification: [`VALIDATION.md`](VALIDATION.md).
- Licence: **LGPL-3.0-only** (rank 1, copyleft — publishable under the same
  licence with its complete source; local-only until the maintainer approves
  publishing, ARCHITECTURE.md §8).

## Layout

```text
powah.hmod            the delivered package (horizon-mod.toml + Cargo.toml + src/ + assets/ + data/)
horizon-mod.toml      the only manifest and identity: id, version 0.2.3, api 0.2, [native] pins
Cargo.toml            one crate; its only dependency is horizon-mod-api from .horizon/sdk/
src/lib.rs            the one `#[mod_api::horizon_mod] impl Hooks`
src/powah.rs          the glue: the three block entity Parts, the queue, the buffers, the cable network, the command
src/runtime.rs        the pure machine rules and their tests (no mod_api types)
assets/powah/         the carried blockstates, models, items, textures and lang
data/powah/           the carried blocks, items, loot, recipes, tags, the creative tab
tools/craft_check.py  the recipe leg: every material and machine recipe in a real crafting table
tools/e2e.sh          the in-game scenario (needs a launcher-built game, see below)
```

## Build and check

The SDK is fetched into `.horizon/sdk/` (gitignored) by the first
`cargo horizon-mod` command; a port that is offline can copy it from another
migrated port at the same pin.

```sh
cd ports/powah
CARGO_INCREMENTAL=0 cargo test                        # the pure rules and the glue's declarations
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check
rm -f Cargo.lock                                      # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build           # -> target/horizon-mod/powah.hmod
cargo horizon-mod check powah.hmod                    # the registry's own check
cargo horizon-mod fits powah.hmod --build nightly     # does it still fit that build
```

## In-game observability

Slice 1 has no GUI, so the port adds two commands (recorded as harness additions,
not upstream gameplay):

```
/powah info <x> <y> <z>          # FE, carbon, burn state, capacity, comparator
/powah set <amount> <x> <y> <z>  # set a furnator's or a cell's stored FE
```

## Running the in-game check

A native package is compiled **into** the client by the launcher, so the check
needs a game built with this package:

```sh
horizon-launcher native-build ports/powah --instance <instance> --trust-native-code
GAME=<that instance's horizon-client> tools/e2e.sh
GAME=<that game> CONTROL_GAME=<an unmodified build> tools/e2e.sh   # the negative leg
SELFTEST=1 GAME=<that game> tools/e2e.sh                           # the fail-capable leg
SELFTEST=craft GAME=<that game> tools/e2e.sh                       # the recipe leg's fail-capable run
SELFTEST=network GAME=<that game> tools/e2e.sh                     # the cable network's fail-capable run
```

Give the port **its own launcher instance** (e.g. `--instance powah-e2e`, and the
same in `HORIZON_E2E_INSTANCE`): an instance that already holds an older install
of the port answers *"was made by an older tool and doesn't say what it uses"* and
leaves the mod out of the build (measured 2026-11-21, `notes.md` §Pin findings 2).
