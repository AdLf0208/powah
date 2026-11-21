# Powah → Horizon — slices 1–3: Furnator, Energy Cell, the Energy Cable network, the dielectric family

Ported from **Powah 6.2.10** (`Technici4n/Powah`, **LGPL-3.0-only**), the exact
ATM10 NeoForge 1.21.1 jar `Powah-6.2.10.jar` (Modrinth SHA-1
`f134ab3e0ace3793abf93129e318eb817898a4e1`, byte-identical to the pack's
`modlist.json` entry; provenance, sizes and hashes in
[`ports/_shared/carry-work/powah/SOURCE.md`](../_shared/carry-work/powah/SOURCE.md)).
Upstream behaviour was read from tag `v6.2.10` (commit
`a5a8f25b5bafb58b694dc44ef0e4c0f14d9e4c11`) — analysis only, this is a
clean-room Rust rewrite. What was read and the numbers extracted:
[`FINDINGS.md`](FINDINGS.md).

**0.2.0 — the SDK package (2026-11-18).** Horizon's mod-API merge retired the `.hnmod`
container, so this port is now **one flat `.hmod`** (mod API 0.2): the carried content
(`assets/`, `data/`) and the Rust (`src/`) ship together, and closed PR **#806** is
superseded. `src/runtime.rs` is **byte-identical** to the pre-migration
`mixins/powah/runtime.rs` (sha256
`91df374d5b925d94fdefe23c4f8b01f164a481cb783b510dcfc50df2284921dc`, including the file inside
the built package). [`## Migration to the SDK format`](#migration-to-the-sdk-format) has the
file map, the injection → shape table, the gaps, the measured ladder and the traps; the retired
tree is `ports/_shared/evidence/powah/retired-hnmod-0.1.0/`. Everything below the migration
section is the 0.1.0 record and stays as the behaviour's history.

**Licence: LGPL-3.0-only → rank 1 (copyleft, publishable).** A port may be
redistributed under the same licence with its complete corresponding source;
the campaign still keeps every port local-only until the maintainer approves
publishing (ARCHITECTURE.md §8). Ported now because it is the next tractable
`content/machines` core mod at licence rank 1 with no unported dependency
(internally it needs nothing but Horizon's own API).

**Class + tier:** `content` · **machines** (`ports/ledger.csv`, mirrored from
`ports/_shared/source-map/atm10-sources.csv`). Approach per the pin's
`docs/modpacks.md` ("What packs need and what runs today"): a **machines**
shape — block entities with ticks, a declared energy capability (API 142), a
container and a command — not a data-only carry. The mod's own tier is
`machines`; nothing here is `deep` or `client`.

## Slice 1 scope

The **starter tier** of the energy chain, one of each:

- **Furnator** (`powah:furnator_starter`): burns furnace fuel into a carbon
  buffer and produces FE into a 20,000 FE buffer;
- **Energy Cell** (`powah:energy_cell_starter`): a 1,000,000 FE capacitor that
  takes and gives FE on every side;
- **Energy Cable** (`powah:energy_cable_starter`): moves FE between the block
  entities beside it, 500 FE/t, storing nothing.

Everything else in Powah (other tiers, creative cell, Magmator, Thermo, Solar,
Reactors, Ender Cells/Gates, Energizing Orb/Rods, Discharger, Hopper, Player
Transmitter, upgrades, GUIs, side config, worldgen) is a recorded gap below.

## Behaviour (upstream → this port)

| Machine | Upstream | This port |
|---|---|---|
| **Furnator** | `AbstractEnergyProvider`: buffer `generatorCapacity` 20,000, transfer 80 a side, generation 20 FE/t | same constants; `runtime::Furnator` |
| fuel | `carbon = burnTime * energy_per_fuel_tick` (**30**), one item at a time, only while carbon is empty; a crafting remainder replaces the stack | same (`load_fuel`, `consume_fuel`, including the remainder — a lava bucket leaves a bucket); the burn time is the engine's `minecraft:cooking_fuel` (`Providers::burn_time`), so modded fuels work. The remainder is not exercised by the e2e |
| production | `min(space, min(generation, carbon))`, and nothing while the buffer is full (carbon untouched) | same (`Furnator::tick`), unit-tested; one coal = 48,000 carbon = 2,400 ticks × 20 FE = **48,000 FE** |
| output | `extractFromSides`: `min(transfer, stored)` into every side that takes it | same, at `level_tick_post` (`push_out`) |
| slot | 2 (0 chargeables, 1 fuel) | 1 (fuel at index 0): charging items needs `item_energy_buffer`, deferred — a recorded deviation |
| **Energy Cell** | `AbstractEnergyStorage`: capacity 1,000,000, transfer 1,000 a side, `Transfer.ALL` | same; the buffer is API 142 (`energy` in the BE's compound) |
| cell tick | push to every side + charge its 2 item slots | push only (no item charging, no slots) |
| **Cable** | no storage (capacity 0); `receiveEnergy` forwards an insertion through the same-tier network, round-robin, skipping the side it came from | no buffer at all; on its tick it moves FE straight from each adjacent giver into each adjacent taker, up to 500 FE/t, with the round-robin start rotating by the game time (`runtime::cable_moves`) |
| cable sides | a side joins a neighbour that is not a cable and has FE | the same rule via `native_energy`; the six visual sides come from the content tag |
| redstone | `checkRedstone()` — default mode `IGNORE`, so always on | no redstone gate (defaults only); a recorded gap |
| comparator | `hasAnalogOutputSignal` from the FE fraction | computed and printed by the command; no `has_analog_output` hook declared — gap |
| keep energy | `keepEnergy()`: the block item carries the buffer | a broken machine loses its FE — gap |

## Hooks (0.1.0, kept as history)

*The retired bundle's wiring. The 0.2.0 mapping is the injection → shape table in
[`## Migration to the SDK format`](#migration-to-the-sdk-format); these injections and helper
files no longer exist.*

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15) | `block_entity.rs` | The three blocks get a block entity. |
| `block_entity_ticks` (15) | `ticks.rs` | All three tick on the server. |
| `block_entity_container` (16) | `container.rs` | The furnator's one fuel slot. |
| `block_entity_energy` (142) | `energy.rs` | Furnator 20,000 / 80 out; cell 1,000,000 / 1,000 both ways; the cable none. |
| `block_entity_tick` (17) | `tick.rs` | The furnator burns; every block queues its position for the move phase. |
| `level_tick_post` (23) | `level_tick_post.rs` | The queued moves (`native_move_energy`): a block is out of the world during its own tick. |
| `register_commands` (8) | `register.rs` | `/powah info`, `/powah set`. |
| `perform_command` (8) | `perform.rs` | The command bodies. |
| `world_lifecycle` (v1) | `world.rs` | Drop the queued positions per world. |

Two pin facts that shaped the design (both recorded by earlier ports and
re-confirmed here):

- API **142** gives a real `Capabilities.Energy.BLOCK` buffer whose amount is
  the int **`energy`** in the BE's saved compound, with `native_energy`,
  `native_energy_insert`/`native_energy_extract` and `native_move_energy`
  (`crates/horizon-client/src/game/machines/native_energy.rs:136,160,174,190`).
  This is what makes the cell a real FE store for other mods.
- `native_energy_at` refuses a block entity **that is ticking at that moment**
  (`native_energy.rs:61-78`), so cross-block moves run at `level_tick_post`,
  where every entity is back in the world.

## Command (harness addition)

`/powah info <x> <y> <z>` prints the block's state, and `/powah set <amount> <x>
<y> <z>` sets a furnator's or a cell's stored FE. They exist because slice 1
has no GUI and no in-game FE readout:

```
Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 46780, burning true, fuel minecraft:coal x4, t=256)
Energy Cell at 2,100,0: 1380/1000000 FE (comparator 0, t=264)
Energy Cable at 1,100,0: no storage, moves up to 500 FE/t (t=188)
```

`t=` is the world game time, so the e2e can tell a fresh reply from a stale one.
`set` is ungated, like `mekanism energy`'s harness control; it is recorded here
as an addition, not upstream gameplay. Upstream's commands are its config/GUI
GUI actions and the guide book; none are ported.

## Content (`.hmod`)

*0.1.0 assembled this into `content/` with `tools/assemble_content.py`; since 0.2.0 the same files
are the package's `assets/` and `data/`, moved with their paths, sizes and hashes unchanged, and
the script is retired with the rest of the old tooling. The patch table below is still the record
of what the carry got wrong; [`CONTENT.md`](CONTENT.md) is the current summary.*

Carried byte-for-byte: the three blockstates, the six models (three variants +
the `furnator`/`energy_cell`/`cable`/`cable_multipart` parents), item models and
26.3 `items/` definitions, seven textures, `en_us.json`, the three loot tables,
the furnator/cell recipes and advancements.

Patched, because the carry cannot get them right (each documented in
`content/README.md`):

| File | Patch |
|---|---|
| `horizon/block/furnator_starter.json` | the carry guessed `minecraft:carved_pumpkin`; the real block is `AbstractGeneratorBlock` with a horizontal `facing` → `minecraft:furnace`, 2.0/20.0, metal sounds, `occludes: false`, stack 1 |
| `horizon/block/energy_cell_starter.json` | the carry guessed `minecraft:stone` (no states); → `minecraft:iron_block` with the upstream numbers, `occludes: false`, stack 1 |
| `horizon/block/energy_cable_starter.json` | the carry **left the cable out** ("no template for properties [down,east,north,south,up,west]") → `minecraft:chorus_plant` + `connects_to: #powah:energy_cable_connects` + `thickness: 4`, empty collision, `occludes: false` |
| `blockstates/furnator_starter.json` | `lollipop:block/empty` (another mod's model) for `facing=up/down`, which the furnace template does not have; now 4 facings × `lit` (never set) |
| `models/block/energy_cell.json` | parent `block` (1.21.1's `models/block.json`) → `block/block`, which 26.3 has |
| `data/minecraft/tags/block/{mineable/pickaxe,needs_iron_tool}.json` | the carry lists all 92 blocks; only the three exist |
| `data/powah/tags/block/energy_cable_connects.json` | new: upstream joins any FE block; a content tag can only name blocks, so the slice's three |
| `data/powah/recipe/crafting/cable_starter.json` + its advancement | the carry skipped them with the block; copied from the jar in the carry's upgraded shape (12 cables) |
| `horizon/creative_tab/machines.json` | upstream's tab is code; one tab with the three items |

The three recipes ship but **do not craft yet**: their ingredients
(`powah:capacitor_basic_tiny`, `powah:dielectric_casing`,
`powah:dielectric_paste`, `powah:dielectric_rod_horizontal`) are later slices.

## Verification (0.1.0, the retired `.hnmod` — historical)

*Everything in this section is the retired bundle's record. The 0.2.0 ladder is in
[`## Migration to the SDK format`](#migration-to-the-sdk-format); the logs it names stay under
`ports/_shared/evidence/powah/` as history and are not the current artifact's evidence.*

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6); checkouts
`ports/_shared/pins/horizon-1d63e0a` and `.../horizon-mixins-5a64af9`. Every
build ran through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`, never
a bare cargo; the e2e used the pin-matched shared `horizon-ctl`
(`ports/_shared/bin/horizon-1d63e0a5/horizon-ctl`) and the port's own fast game.

### Ladder

| Step | Command | Result |
|---|---|---|
| format | `cargo fmt` (through the same rustfmt the ladder uses) | clean |
| runtime tests | `cargo test --locked --offline` (standalone lib) | **14 passed**, 0 failed; TDD: the same 14 were watched failing first (11 failed, 3 "nothing happens" assertions trivially held on stubs) |
| staged check | `python3 tools/horizon.py check --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, **0 warnings** |
| embedded tests | `python3 tools/horizon.py test …` | `14 passed; 0 failed; 1240 filtered out` |
| fast build | `python3 tools/horizon.py build …` | `cargo_exit: 0`, binary in the slot target |
| e2e (one delivered file) | `EMBEDDED=1 tools/e2e.sh` on the port's fast game, `--dev-mod` omitted | **PASS**, `EXIT:0`; the content came from the `.hnmod`'s embedded copy, not `content/` |
| fail-capability | `env EMBEDDED=1 SELFTEST=1 tools/e2e.sh` | **exit 1** at the flipped no-fuel assertion |
| content `.hmod` | `cargo horizon-mod build` + `check` | **39,262 B**, SHA-256 `7d344174…26c1`, **42 entries**, content only, API 0.2, `side = both` |
| native `.hnmod` | `tools/package.sh` (content → `mixins/powah/data/powah-content.hmod` → `.hnmod`) | **33,899 B**, SHA-256 `0030419c…a9cdd`, **15 entries**, including the embedded content |
| load-check | `tools/hmod-load.sh` (shared 1d63e0a5 game) | **PASS** with `--dev-mod <the .hmod>` (three blocks + three items); without it the same ids are **unknown** (`Unknown block type 'powah:furnator_starter'`, block stays bedrock) |

Evidence: `ports/_shared/evidence/powah/{e2e,e2e-selftest,hmod-load,horizon-check,horizon-test,horizon-build}.log`.

**One delivered file.** The registry stores one file per project, so the built
content mod is carried inside the bundle at
`mixins/powah/data/powah-content.hmod` and the `embedded-content` injection at
`content_root` writes it to `.horizon-cache/` and appends it to the dev-mod list
(the AE2 pattern, `docs/native-mods.md` "Data files"). The e2e above runs with
no `--dev-mod`, so it proves the delivered `.hnmod` installs both halves;
`content/powah.hmod` stays as the content input and as the standalone
`hmod-load.sh` check.

### In-game scenario (quoted from `e2e.log`)

```
  arena: y=100.0 floor(0,99,3)=minecraft:stone feet(0,100,3)=minecraft:air
  block(0,100,0) = powah:furnator_starter
  block(1,100,0) = powah:energy_cable_starter
  block(2,100,0) = powah:energy_cell_starter
  furnator: Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, t=150)
  after 40 ticks: Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, t=199)
  has(0,100,0) minecraft:coal x4: ok
  has(0,100,0) minecraft:coal x4: ok
  furnator: Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 46800, burning true, fuel minecraft:coal x4, t=282)
  cell: Energy Cell at 2,100,0: 1360/1000000 FE (comparator 0, t=290)
  carbon 47800 -> 46800, cell 1360 FE after 40 burning ticks
  set(2,100,0) = Energy Cell at 2,100,0: 0/1000000 FE (comparator 0, t=305)
  no cable: furnator 320 FE, cell Energy Cell at 2,100,0: 0/1000000 FE (comparator 0, t=363)
  set(2,100,0) = Energy Cell at 2,100,0: 1000000/1000000 FE (comparator 15, t=380)
  after 40 ticks: Energy Cell at 2,100,0: 1000000/1000000 FE (comparator 15, t=429)
  set(2,100,0) = Energy Cell at 2,100,0: 0/1000000 FE (comparator 0, t=438)
  block(1,100,0) = powah:energy_cable_starter
  cell: Energy Cell at 2,100,0: 3920/1000000 FE (comparator 0, t=494)
  set(2,100,0) = Energy Cell at 2,100,0: 1000/1000000 FE (comparator 0, t=509)
  block(2,100,1) = powah:energy_cable_starter
  block(2,100,2) = powah:energy_cell_starter
  source: Energy Cell at 2,100,0: 500/1000000 FE (comparator 0, t=572)
  sink:   Energy Cell at 2,100,2: 500/1000000 FE (comparator 0, t=580)
PASS: a fuelled furnator made FE, the cable carried it into the cell, an unfuelled furnator made nothing, a cell without a cable stayed empty, a full cell stayed full, and a charged cell drained through a cable into an empty one
EXIT:0
```

The run's controls: a **no-fuel** furnator stores 0 FE for 40 ticks (and the
cell beside it stays 0); the same furnator with the **cable broken** holds its
FE while the cell stays 0 (so the cable is what moves it); a **full cell**
stays at 1,000,000 FE while the furnator keeps burning; a charged cell is
drained through a cable into a second cell. `SELFTEST=1` flips the no-fuel
assertion and the run fails at it (`FAIL: SELFTEST flipped: the unfuelled
furnator made no FE, as it should`, exit 1), so the scenario can fail. The
48,000-FE-per-coal figure is pinned by the unit test, not by the e2e (the chat
round trip in this environment is ~2 s, far longer than a tick).

## Deviations and gaps (slice 1)

1. **No GUI.** Upstream opens a container screen for the furnator/cell/cable;
   this slice inserts fuel through the container (automation's path, what the
   e2e uses) and reads state with `/powah info`.
2. **No cable network** — **closed by slice 3** (`## Slice 3 — the cable network`); the
   slice's gap line, kept verbatim: Upstream's cables share one buffer across a connected
   network and forward an insertion through it in one call. The port moves FE
   one cable hop per tick between *adjacent block entities*, so a run of two or
   more cables does not relay. The e2e's chain is exactly one cable (the slice's
   brief).
3. **Two adjacent cells swap energy.** Both push on every side by default (as
   upstream's `Transfer.ALL` does), so a pair exchanges its charge each tick and
   the later tick holds it. The port is faithful; upstream's answer is the side
   config GUI, which is not ported. The e2e avoids the shape (cell → cable →
   cell).
4. **No item charging.** Upstream's furnator charges slot 0 and a cell charges
   two slots through `item_energy_buffer`; not ported, so the furnator has one
   slot, not two, and the cell none.
5. **Breaking a machine loses its FE.** Upstream's `keepEnergy()` makes the
   block item carry the buffer; no energy component/item buffer is registered
   here, and the loot tables drop the plain block.
6. **No side config or redstone modes.** Upstream's six per-side transfer types
   and redstone IGNORE/ON/OFF default to `ALL`/`IGNORE`; this slice ships only
   those defaults.
7. **No comparator output.** `comparator_strength` is computed and printed, but
   no `has_analog_output` hook is declared, so a vanilla comparator beside a
   machine still reads container fullness.
8. **`lit`/`burning` is not a block state** (upstream's `burning` is a client
   sync flag for particles and sound), so nothing is drawn differently while
   burning, and no particles/sound play.
9. **No item count/stack parity for the cable's visual sides**: the content tag
   names the slice's three blocks; upstream also draws arms to any other mod's
   FE block.
10. **Crafting is not possible yet**: the recipes need later-slice items.
11. **The other 6 tiers, the creative cell and ~15 machines** are unported. — the
    **energy cell** tiers are closed by [`## Slice 4 — the energy cell ladder`](#slice-4--the-energy-cell-ladder-2026-11-21);
    the furnator and cable tiers, the creative cell and the machines remain.
12. **`/powah set` is a harness control**, not upstream gameplay (same precedent
    as `/mekanism energy`).

## Pin watch (2026-11-09 ruling)

`python3 ports/tools/check-pin-updates.py` reports the pin **moved** (Horizon
`7d0f8b956b09`, 2,439 commits ahead; catalog +2 hooks `world_hold`,
`world_warp`, +3 chunk helpers). None of them touches this slice's surface
(API 8/15/16/17/23/142 are unchanged), and the brief pins `1d63e0a`, so the
port stays on it; the repin decision is the orchestrator's.

## Harness findings (for the next machine port)

- `NativeCommand` argument kinds are `word`/`text`/`integer`/`number`/
  `boolean`/`player`/`item`/`block_pos` (`docs/native-mods.md:2173`); `int` is
  refused with "unknown kind `int`" and the whole usage is dropped.
- The container path (`ctl container X Y Z item n`) inserts into `inputs` on
  the next tick, and the furnator consumes the first fuel item immediately, so
  read `n-1` back, not `n`.
- Two mod block entities tick in the same level tick; a cross-block energy move
  must be queued per position and run at `level_tick_post` — a per-tick read of
  a neighbour inside a BE tick returns nothing (`native_energy_at` refuses the
  ticking entity).
- `ctl chat` replies take ~2 s (~40 ticks) in this environment; assert on a
  fresh `t=` field, and for a value you set, poll until it reads back.
- The `.hmod` load-check must launch the **shared unmodded game** for the
  negative leg; the port's own build already has the native half.
- A model parent of `block` (1.21.1's `models/block.json`) is missing in 26.3
  and logs "missing block model minecraft:block"; use `block/block`.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Review repairs (PR #554)

The reviewer's blocker is staging (the pinned artifact absent from the registry
release/staging), which is the orchestrator's (horizon-mods#686). The licence is
LGPL-3.0-only, the original's, with the complete corresponding source in this
repository — the registry policy accepts copyleft; the reviewer's inability to
read LGPL source under CONTRIBUTING is a reviewer-permission matter, not a
listing defect. This touch adds the notices to the artifact
(`mixins/powah/LICENSE` and `NOTICE.md`, and the same at the embedded content
root) and repackages it.

### Verification after the repairs

- Artifact: `powah.hnmod`, 40,622 B, SHA-256 `2dabb27b01f31e201903fd5f55d42d86d92aed816d3967701d23e06f134d2b42` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/powah.hmod`).
- `upstream_ready.py powah --copyleft` exits 0 (READY, LGPL-3.0-only). `check-skills.py --notes ports/powah` exits 0.

## Migration to the SDK format

*Recipe followed from `ports/imfast/notes.md` → `## Migration to the SDK format` (the first
migration), with `ports/charginggadgets/notes.md` (a block entity, a container, an energy buffer,
typed commands) and `ports/laserio/notes.md` (the per-tick queue drained at `level_tick_post`)
for the parts this port shares. 2026-11-18. Version **0.1.0 → 0.2.0**; **PR #806** (the
`.hnmod`) is superseded.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired the `.hnmod`
container, and the registry closed every `.hnmod` submission on 2026-11-18 with one reason:
*"republish this as a native `.hmod` with the SDK from the next Nightly"*. The unit is **one flat
`.hmod`** — content, native code, or both — so this port no longer needs the AE2 embedded-content
trick, its `content_root` injection or its `.horizon-cache` write: `assets/` and `data/` sit
beside `src/` in the same package.

**Pins.** SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (surface + `api-source`
cached at `ports/_shared/sdk/0b14f8a/`, copied into `.horizon/sdk/` for the offline builds),
mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`** (was `5a64af93…`), `package_format` 6,
`content_api` 0.2. The old campaign pin `1d63e0a` does not apply to a package build.

**The retired tree was moved before the first build** (trap 1): `mixins/`, `powah.hnmod`,
`compatibility.json`, `rust-toolchain.toml`, the port's old `AGENTS.md` (it told the reader to
run `tools/horizon.py` and read `compatibility.json`), `.github/`, `content/` (its
`horizon-mod.toml`, `powah.hmod`, `README.md` and `target/`) and the eight retired tools
(`horizon.py`, `package.py`, `package.sh`, `assemble_content.py`, `add-notices.py`,
`test_stage.py`, `test_template.py`, `hmod-load.sh`, `tools/audit/`) are in
`ports/_shared/evidence/powah/retired-hnmod-0.1.0/`. `Cargo.lock` was deleted, not kept.

### File map (0.2.0)

| 0.1.0 | 0.2.0 | Evidence |
|---|---|---|
| `mixins/powah/runtime.rs` | `src/runtime.rs`, copied with `cp` and proved with `cmp` | retired copy kept; all three copies sha256 `91df374d…21dc` |
| `mixins/powah/support.rs` (`powah_state`, 451 lines) | `src/powah.rs` — the same read/write/push/relay/readout logic over `&mut dyn ServerHost` | — |
| `mixins/powah/horizon-mixin.toml` (2 modules, 10 injections) | no plan: `Registrar` declarations, two Part types and four `Hooks` methods | retired |
| `mixins/powah/*.rs` (10 helper files) | the `Hooks`/Part methods and the declarations below; no per-hook files | retired |
| `content/assets/`, `content/data/` | `assets/`, `data/` at the package root | 41 files, byte-identical (`carried-content-sha256.txt`) |
| `content/horizon-mod.toml` | merged into the root `horizon-mod.toml` (`[native]` added; version 0.2.0) | retired |
| `content/powah.hmod` (39,262 B content carry) | not needed: the one package carries the content | retired |
| `mixins/powah/data/powah-content.hmod` + `embedded_content.rs` | gone: nothing to embed | retired |
| `compatibility.json`, `rust-toolchain.toml`, `horizon-native.json` | `[native]` in `horizon-mod.toml`; the pinned commit decides the toolchain | retired |
| `powah.hnmod` (40,622 B) | `powah.hmod` (68,403 B, native + content) | retired |
| `tools/horizon.py`, `package.*`, `assemble_content.py`, `hmod-load.sh`, `tools/audit/`, `.github/` | retired; the content carry is recorded in `CONTENT.md` | retired |

**Kept unchanged:** `FINDINGS.md`, `CONTENT.md`, `VALIDATION.md`, `LICENSE`,
`notes.md` (this section added), and `tools/e2e.sh`'s *scenario* — its assertions are
format-independent; only the launch line changed (a launcher-built `GAME=`, an optional
`CONTROL_GAME=`, no `--dev-mod`).

**New:** `horizon-mod.toml`, `Cargo.toml` (one crate, `edition 2024`, the only dependency
`horizon-mod-api` from `.horizon/sdk/`), `src/lib.rs`, `src/powah.rs`, `.gitignore`
(`/target`, `/.horizon`, `/Cargo.lock`), a rewritten `README.md`, an updated
`LICENSE-NOTICE.md`.

### Manifest mapping

| 0.1.0 (plan + content manifest) | 0.2.0 (`horizon-mod.toml`) |
|---|---|
| `[mod] id = "powah"` | `id = "powah"` (also the resource namespace) |
| `[mod] version = "0.1.0"` and content `6.2.10` | `version = "0.2.0"` — the port's version, one for the one package |
| `[mod] priority = 100` | gone; ordering would be `#[listener(priority = …)]` |
| `[compatibility] horizon_revision = "1d63e0a5…"` | `[native] horizon = "0b14f8ac…"` |
| mixins `5a64af938ec8…` (the bundle) | `[native] mixins = "e59da9094f0c…"` — **it moved** |
| game 26.3 (content manifest) | `[native] game = "26.3"` |
| `[[module]] runtime` + `state` | ordinary modules: `pub mod runtime;`, `mod powah;` |
| 10 `[[injection]]` | 4 `Hooks` methods + 3 `BlockEntityPart` methods + 3 declarations (table below) |
| — | `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]` |

### Injection / helper → shape (every one mapped, none silently dropped)

| 0.1.0 injection (API) | 0.2.0 shape | Kept / gap |
|---|---|---|
| `embedded-content` (`content_root`, raw target, HEAD, cancellable) | — | **gone by design**, not a gap: one `.hmod` carries `assets/`, `data/` and `src/`, so there is nothing to write out or append to the dev list |
| `block-entity` (`block_entity` 15) | the three **types** declared in `Hooks::register`: `block_entity_type("furnator_starter"/"energy_cell_starter"/"energy_cable_starter").blocks(&[..]).build()` | **kept as a declaration**: a declared type starts with an empty compound, so no Part method is needed (`BlockEntityPart::block_entity` stays unset) |
| `block-entity-ticks` (15) | `BlockEntityPart::block_entity_ticks` on both Part types → `event.ticks = true` | **kept** |
| `block-entity-container` (16) | `Registrar::block_container(&furnator, container())` — 1 slot, every face in and out, `menu: None` | **kept** |
| `block-entity-energy` (142) | `Registrar::block_energy` for the furnator (20,000; insert 0; extract 80; extract faces only) and the cell (1,000,000; 1,000 both ways, all faces); **the cable declares none** | **kept**, keyed by the block instead of a shared `Option` |
| `block-entity-tick` (17) | `FurnatorPart::block_entity_tick` (load fuel, burn, write, queue) and `IdlePart::block_entity_tick` (queue only) | **kept**; the reply is `Unchanged` when nothing moved |
| `level-tick-post` (23) | `Hooks::level_tick_post` over the `LevelRef`-keyed queue: `push_out` for the furnator/cell, `relay` for the cable | **kept** |
| `register-commands` (8) | `Hooks::register_commands` with typed usages (`<pos:block_pos>`, `<amount:integer>`) | **kept** |
| `perform-command` (8) | `Hooks::run_mod_command` reading `literals` and the named arguments | **kept**, minus the free-text parsing (trap 9) |
| `world-lifecycle` (v1) | `Hooks::world_lifecycle` → `Queue::clear` | **kept** |

So: **9 named hooks + 1 raw target → 4 `Hooks` methods, 3 Part methods and 3 declarations; 10
mapped, 0 recorded as gaps.**

Host calls, 0.1.0 → 0.2.0:

| 0.1.0 (`Game<'_>`) | 0.2.0 |
|---|---|
| `game.res.providers().burn_time(items, stack, false)` | `host.item_burn_time(stack.item())` |
| `items().get(&stack.item).remainder` | `host.item_crafting_remainder(stack)` (then `ItemStack::shrink(1)`) |
| `game.native_energy(dimension, pos, side)` | `level.capability(host, &ENERGY, pos, Some(side))` (`amount`, `capacity`; the moves themselves are `insert`/`extract`) |
| `game.native_move_energy(..)` | `move_energy(host, &source, &target, max, tx)` in `host.transaction(..)` |
| `game.native_with_mod_block_entity(pos, f)` | `host.block_entity(level, pos)` + `BlockEntityRef::{data, item_slot, merge_data}` |
| `game.world.game_time` | `level.game_time(host)` |
| `game.world.modified.insert(chunk)` | nothing: the tick reply dirties the chunk, `Changed` only when something moved |
| `game.native_feedback(Text)` | `host.commands().feedback(Text)` |
| `game.native_dimension()` | the queue is keyed by `LevelRef` (Copy + Hash + Eq), so no dimension strings |
| `game.world.block_entities.get(&key)` | `host.block_entity(level, pos)` / `level.block(host, pos)?.id(host)` |

### Ladder measured for 0.2.0

Every step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job powah-sdk --port ports/powah --step … -- bash -c 'cd
ports/powah && …'` lease (the runner does not `cd`, and an SDK package has no staged tree). Logs in
`ports/_shared/evidence/powah/sdk-*.log`.

| Step | Command | Result |
|---|---|---|
| runtime carried over | `cp` + `cmp` + `sha256sum` | `91df374d…21dc` in the retired copy, `src/runtime.rs` and inside the package |
| runtime tests before any glue | `cargo test` with `src/lib.rs` = `pub mod runtime;` | **14 passed**, 0 failed (`sdk-runtime-first.log`) |
| the glue's tests can fail | 8 new tests, every body `todo!()` | **14 passed, 8 failed**, each `not yet implemented` (`sdk-glue-red.log`) |
| all tests | `cargo test` | **22 passed**, 0 failed — 14 runtime + 8 glue (`sdk-test.log`) |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` (never `--all`: trap 6) | exit 0, empty output (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock; cargo horizon-mod build` | `target/horizon-mod/powah.hmod` (`sdk-build.log`) |
| determinism | `rm -f Cargo.lock; cargo horizon-mod build` again | **byte-identical**, same 68,403 B and sha256 (`sdk-build-second.log`) |
| a clean tree packs the same bytes | `rm -rf target`, then `rm -f Cargo.lock; cargo horizon-mod build` | same 68,403 B, same sha256 (`sdk-fresh-tree.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | see the verbatim output below, **EXIT 0** (`sdk-check.log`) |
| fits the pinned build | `cargo horizon-mod fits powah.hmod --build nightly` | `Powah: 0b14f8a  made for`, **EXIT 0** (`sdk-fits.log`) |
| what it uses | the package's `uses.json` | **310** entries (`sdk-uses-summary.log`): 25 hook, 33 method, 24 trait, 146 type, 71 handle, 5 part, 2 capability, 2 macro, 2 fn |
| package contents | `unzip -l powah.hmod` | **50 entries**: 6 root documents (`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `uses.json`), 3 `src/`, 25 `assets/`, 16 `data/`; **no `Cargo.lock`** (`sdk-entries.log`) |
| carried content | `sha256sum` of `assets/` + `data/` before and after the move | **41 files, unchanged** (`carried-content-sha256.txt`) |

`cargo horizon-mod check powah.hmod`, verbatim:

```text
powah 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  68403 bytes, SHA-256 11824c7cc17003caf71f1ee9eef63bf1fdfc81c1c2535b7fa54463ce3166455d, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`cargo horizon-mod fits powah.hmod --build nightly`, verbatim:

```text
Powah:
0b14f8a  made for
```

### Packages

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `powah.hmod` (0.2.0, the delivered package) | 68,403 B | `11824c7cc17003caf71f1ee9eef63bf1fdfc81c1c2535b7fa54463ce3166455d` | 50 |
| `powah.hnmod` (0.1.0, retired) | 40,622 B | `2dabb27b01f31e201903fd5f55d42d86d92aed816d3967701d23e06f134d2b42` | 13 |
| `content/powah.hmod` (0.1.0 content carry, retired) | 39,262 B | `7d344174…26c1` | 42 |

The manifest the tool packs is the one written (no field was stripped): the package's
`horizon-mod.toml` is byte-identical to the port's, comments included.

### Gaps this migration adds

1. **No in-game run at 0.2.0.** `horizon-launcher` is not installed here, so no `0b14f8a` game
   carrying this package exists (an SDK package is compiled **into** the client; there is no
   staging path). `tools/e2e.sh` was rewritten for a launcher-built `GAME=`, an optional
   `CONTROL_GAME=` (an unmodified build of the same version, where all three ids and `/powah`
   must be unknown) and the same fail-capable `SELFTEST=1`; it refuses to start without `GAME`
   (measured: `missing GAME: …`, exit 1) and was **not run**. The 0.1.0 runs remain the
   behaviour's evidence; the byte-identical `runtime.rs`, the same saved keys (`energy`,
   `carbon`), the same numbers and the same readout lines make it the same behaviour **except**
   what only a game can show: (a) a declared block entity type gives the furnator its one
   container slot and the energy capability reads the compound's `energy`; (b) a move between two
   block entities works at `level_tick_post`; (c) the cable's 500 FE/t budget and round-robin are
   unchanged; (d) `/powah info|set` print and act as before.
2. **`/powah set` no longer explains a bad amount.** `<amount:integer>` is parsed (and refused) by
   the dispatcher, so 0.1.0's *"The amount must be a whole number of FE."* line is gone.
3. **A command acts on the level in place.** `/powah info|set` read `host.level(host.dimension())`,
   as 0.1.0 read `game.world`; a command sent with `execute in <other dimension>` still acts on the
   ticked level. Single player and a LAN host are unaffected; recorded rather than worked around.
4. **A foreign FE block's per-call limits and faces are invisible.** `EnergyHandler` exposes
   `amount`/`capacity` and the moves only, so the cable's planner reads the limits back from this
   mod's own declaration and treats a foreign block as unbounded; its handler then caps the move
   (and `move_energy` retries with what it took, so nothing is lost). No other FE block exists in
   this slice, so nothing observable changes.
5. `/powah set` is operator-only since the 2026-11-18 review (see `## Review fixes`), as
   `laserio`'s mutating commands became at review; `info` too, because its fuel readout
   discloses another player's machine contents.
6. Breaking a machine still loses its FE (0.1.0 gap 5), and the slice boundary is unchanged: no
   GUI, no cable network, no item charging, no side config, no comparator hook, the other six
   tiers unported.

### Traps this migration hit (1–15 are the reference's; 16–25 are this port's)

1. **A leftover old path refuses the package.** The retired tree must be out of the port root
   **before** the first build; here it was `mixins/`, `content/`, `powah.hnmod`, the old
   `Cargo.lock` and the old tools.
2. A short pin reads as *missing*: both pins are 40 hex, and the **mixins commit moved**
   (`5a64af93…` → `e59da9094f0c…`).
3. `cargo horizon-mod check` with **no file** checks the wasm target; the package check is
   `check <file.hmod>`.
4. `cargo fmt --all` reaches into `.horizon/sdk/`; use `cargo fmt --check`.
5. `rm -f Cargo.lock` immediately before `cargo horizon-mod build` — any `cargo test`/`clippy`
   writes one and a packed lock changes the artifact.
6. `runtime.rs` must stay byte-identical (`cmp` **and** `unzip -p … src/runtime.rs | sha256sum`);
   `pub mod runtime;` is required or every `pub` item dead-codes.
7. `side = "both"` (an integrated server is part of the client installation) and
   `capabilities = ["registries"]` are mandatory values, not claims.
8. **The three types get two Part types, not three:** the furnator burns, the cell and the cable
   only queue. Nothing in the glue reads `event.entity.id()`: the attached Part decides what the
   tick does, and the move phase asks the **block** id, which is the content's own.
9. **`perform_command` is gone → `run_mod_command` with typed arguments.** The old
   `split_whitespace` parse (and its two usage lines) goes with it; the **readout lines stay
   byte-identical** so the 0.1.0 e2e assertions still match.
10. **`host.transaction` comes from `HostTransaction`**, not from `ServerHost` itself: without
    `use mod_api::HostTransaction;` the call is E0599 (`no method named transaction`).
11. **`move_energy` is the primitive, not a hand-rolled pair.** It extracts, inserts, and when the
    target takes only part it aborts that transaction and retries with the accepted amount — so a
    per-call cap (the furnator's 80, a cell's 1,000) never loses energy and nothing needs the
    insert-then-extract dance `bigreactors` does.
12. **The energy capability has no limit or face getters.** `EnergyHandler` is
    `amount`/`capacity`/`insert`/`extract`; the cable's planner needs `max_insert`,
    `max_extract`, `can_insert` and `can_extract`, so they come from the mod's own declaration
    (`battery_of`), the same table `Registrar::block_energy` uses. One table, so the declaration
    and the relay cannot disagree; a foreign block is planned as unbounded (its handler caps).
    Without this, a cable between a cell and a furnator would spend its budget planning a move
    *into* the furnator (which refuses inserts) and carry less than it should.
13. **A cable that stores nothing still needs a declared type and a tick.** The SDK cannot
    enumerate a level's block entities and a content block has no server tick, so the queue is the
    only way to find the cable — and the queue is keyed by `LevelRef`, not by a dimension string.
14. **`BlockEntityTickReply` arrives as `Changed`.** The dispatch sets `Changed` before each Part,
    so a Part that changed nothing must set `Unchanged` itself, or every machine dirties its chunk
    every tick (and clients get an update every tick).
15. **A declared type starts with an empty compound**, so `BlockEntityPart::block_entity` is not
    needed (`charginggadgets` returns `Some(Compound::new())` redundantly); the saved keys appear
    on the first tick.
16. **The old `Cargo.toml` cannot be carried over.** It had `[workspace] members = ["tools/audit"]`,
    `[lib] path = "mixins/…"` and a `serde_json` dependency — a path that no longer exists, a
    package path the tool refuses, and a registry dependency that would need
    `horizon-launcher native-lock`. The new crate has one dependency: the SDK.
17. **`cargo test` needs `.horizon/sdk/` before the first `cargo horizon-mod` command.** Copied
    from a port at the same pin (`cp -a ports/framedblocks/.horizon/sdk ports/powah/.horizon/sdk`);
    the SDK also fetches itself on an online build.
18. **`BlockEntityTypeBuilder::blocks` takes bare names.** `.blocks(&["furnator_starter"])` names
    `powah:furnator_starter`; an id without its namespace would silently attach nothing.

Skills: superpowers/using-superpowers,writing-plans,test-driven-development,systematic-debugging,verification-before-completion (the recipe is fixed and no design choice was open, so `brainstorming` did not apply to this migration; the 0.1.0 record above lists it); BeautifulRust/rust-best-practices,rust-port

### Changelog / supersede text (for the resubmission that replaces #806)

> **0.2.0 — republished as a native `.hmod` (the SDK format).** Horizon's mod-API merge retired
> the `.hnmod` container, so this release replaces the closed submission #806 with one flat
> `.hmod` that carries the starter Furnator, Energy Cell and Energy Cable — their blocks, items,
> models, textures, recipes, loot, tags and lang — **together with the code**. The embedded-content
> workaround of 0.1.0 (a second `.hmod` inside the bundle, written to `.horizon-cache` at start) is
> gone: one file installs both halves. Built against SDK nightly `0b14f8a` (mixins `e59da90`, api
> 0.2, format native-1). Behaviour is unchanged: a fuelled furnator burns furnace fuel into carbon
> and makes 20 FE/t into a 20,000 FE buffer (one coal is 48,000 FE), the energy cell is a
> 1,000,000 FE capacitor that takes and gives 1,000 FE a side, and the energy cable moves up to 500
> FE/t between the block entities beside it, storing nothing; the buffered machines declare a real
> `Capabilities.Energy.BLOCK` buffer that other machines can fill and drain, and `/powah info|set`
> print and set the same lines. What changed is the wiring: the ten injected helpers became four
> `Hooks` methods, three `BlockEntityPart` methods and three declarations (the block entity types,
> the furnator's container, the two energy buffers), and a declared buffer plus a typed command
> pair replace the old hooks' shared outputs. `src/runtime.rs` is byte-identical to 0.1.0 and keeps
> its 14 tests. Version `0.1.0` → `0.2.0`.

## Review fixes (2026-11-18)

Source review of PR **#1222** (`var/review/1222-powah.md`), both findings fixed in **0.2.1**.
Both are the same defect: the two harness usages are administrative — they act on any loaded
machine at coordinates the caller chooses — and 0.2.0 declared them with `Command::new` (level 0).

| # | Finding (`file:line`) | What a player could do | Fix | Test that pins it |
|---|---|---|---|---|
| 1 | `src/lib.rs:121` — `set` mutates any loaded cell/furnator | a non-operator overwrote a machine's FE at any coordinates in the level: create energy without fuel, drain another player's machine | `powah::declare_commands` registers `powah set <amount> <pos>` with `Command::op(…, 2)`; the server's dispatcher refuses it below game-master permission, before any listener runs | `powah::tests::the_harness_commands_are_operator_only` (`src/powah.rs`) — RED on the 0.2.0 registration (`left: 0, right: 2`, log `evidence/powah/review-test-red.log`), GREEN after |
| 2 | `src/powah.rs:239` — `info` discloses another player's fuel | a non-operator read the fuel item and stack count of any loaded furnator at any coordinates | same `Command::op(…, 2)` for `powah info <pos>`; it is operator diagnostics, not gameplay (upstream's own readout is its GUI, which is not ported) | the same test iterates **both** declared usages, so dropping `info` back to level 0 fails it |

No upstream defect was involved; this is a **port** defect (the commands are the harness surface
0.1.0 added, and 0.2.0 kept their level at 0). The commands' words, argument kinds and reply lines
are unchanged, so every readout the e2e asserts is byte-identical.

**The in-game scenario changed in one place, and got stronger.** At `0b14f8a`, `ctl container`
reads a mod block entity's raw `items` vector, which the engine grows only through the game's own
container command path (`item replace block … container.N …`, the path a hopper uses) or an opened
menu — so `ctl container 0 100 0 minecraft:coal 5` reported *"container is full"* with no slots
(first run: `evidence/powah/first-run-container.log`). The e2e now inserts through
`item replace block 0 100 0 container.0 with minecraft:coal 5` and reads the slot back from
`/powah info`'s own `fuel <item> x<count>` field (`fuel_is`), i.e. the port's saved data rather
than a harness copy. Every assertion it had is kept, and the stack-shrink check is now read from
the machine itself.

### Verification (0.2.1)

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test` | **23 passed**, 0 failed (`evidence/powah/sdk-test.log`) |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, no warning (`evidence/powah/sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`evidence/powah/sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical**, 68,970 B (`evidence/powah/sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.1 … 68970 bytes, SHA-256 698b63fb…981670` (`evidence/powah/sdk-check.log`) |
| fits the build | `cargo horizon-mod fits powah.hmod --build nightly` | `Powah: / 0b14f8a  made for`, **exit 0** (`evidence/powah/sdk-fits.log`) |
| in-game | `ports/tools/launcher-game.sh run powah` | `PASS: …` + `EXIT:0` (`evidence/powah-e2e/e2e-launcher.log`) |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME … bash ports/powah/tools/e2e.sh` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`…-control.log`) |
| selftest | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`…-selftest.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | `logs=3 ok=3 missing-exit=0 bad-exit=0`, exit 0 |

The e2e now runs on a launcher-built game carrying this package (the gap the 0.2.0 notes
recorded), including the **control** leg: the same Nightly without `powah` knows neither block id,
so the behaviour is the package's. Package: `powah.hmod`, **68,970 B**, sha256
`698b63fb7880f84729dda23d0c9a14ccca091ebd595b97a7c56b977efc981670`.

## Slice 2 — the dielectric/capacitor crafting family (2026-11-20)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 10 — the notes' own next-slice authority:

> **10. Crafting is not possible yet**: the recipes need later-slice items.

and `## Content (`.hmod`)`, the sentence that names them by id:

> The three recipes ship but **do not craft yet**: their ingredients
> (`powah:capacitor_basic_tiny`, `powah:dielectric_casing`, `powah:dielectric_paste`,
> `powah:dielectric_rod_horizontal`) are later slices.

Item 11 ("The other 6 tiers, the creative cell and ~15 machines are unported") names no single
next machine, and every one of those tiers' recipes needs this same material family first, so
the **smallest complete family the notes name concretely** is the one in gap 10: the
dielectric/capacitor items. This slice ships that family and, with it, makes the three carried
machine recipes craft — which is what the 2026-11-20 parity rule (AGENTS.md) requires of every
carried recipe.

**Chosen:** the crafting-material family (six items, eight recipes). **Left:** the six machine
tiers (`furnator_basic` … `nitro`), the creative cell, the cable network (gap 2), item charging
(gap 4), side config/redstone (gap 6), the comparator hook (gap 7), the GUIs (gap 1) and
`powah:capacitor_basic_large` (its consumers are `battery_basic`, `reactor_basic` and the higher
capacitor tiers — none of them in this slice).

### What it ships

| Family | Files |
|---|---|
| items (6) | `data/powah/horizon/item/{dielectric_paste,dielectric_rod,dielectric_rod_horizontal,dielectric_casing,capacitor_basic,capacitor_basic_tiny}.json`, each `{"template": "minecraft:stick"}` (upstream's plain 64-stack items) |
| item models (6) | `assets/powah/models/item/<id>.json` carried from the jar (`dielectric_casing` is its Blockbench 3D model on `block/block`) |
| item definitions (6) | `assets/powah/items/<id>.json` in the 26.3 shape the three existing items already use |
| textures (6) | `assets/powah/textures/item/<id>.png`, carried byte-for-byte |
| recipes (8) | `data/powah/recipe/crafting/{dielectric_paste,dielectric_paste_2,dielectric_rod,dielectric_rod_2,dielectric_rod_h,dielectric_rod_h_2,capacitor_basic,capacitor_basic_tiny}.json` |
| advancements (8) | the jar's recipe-unlock files under `data/powah/advancement/recipes/misc/crafting/` |
| creative tab | the six items join `data/powah/horizon/creative_tab/machines.json` (upstream has one Powah tab) |
| lang | already carried: `item.powah.*` names are in `assets/powah/lang/en_us.json` |

Upstream's numbers (jar `data/powah/recipe/crafting/`, read at tag `v6.2.10`):

| Recipe | Shape | Out |
|---|---|---|
| `dielectric_paste` | shapeless: 3 × `#minecraft:coals` + 2 × `minecraft:clay_ball` + `minecraft:lava_bucket` | 24 |
| `dielectric_paste_2` | shapeless: 2 × `#minecraft:coals` + `minecraft:clay_ball` + `minecraft:blaze_powder` | 16 |
| `dielectric_rod` | shaped `pip/pip/pip` (`p` paste, `i` iron bars) | 8 |
| `dielectric_rod_h` | shaped `ppp/iii/ppp` | 8 |
| `dielectric_rod_2` | shapeless: one horizontal rod → one vertical rod | 1 |
| `dielectric_rod_h_2` | shapeless: one vertical rod → one horizontal rod | 1 |
| `capacitor_basic` | shaped ` ip/iri/pi ` (`i` iron ingot, `p` paste, `r` redstone block) | 4 |
| `capacitor_basic_tiny` | shapeless: one `capacitor_basic` | 2 |

No Rust changes: these are plain items, and a plain item is a `horizon/item` declaration plus a
model, exactly as `docs/mods.md` §"Content mods (API 0.2)" says. `src/runtime.rs` and
`src/powah.rs` stay byte-identical to 0.2.1, so their 23 tests are unchanged.

### How it is proven (parity rule)

`tools/craft_check.py` drives a real crafting table through `horizon-ctl` (`screen` slots
100–108 row-major, result 200, the `additional_lights` pattern): for **each of the eight
material recipes and the three machine recipes** it gives the declared ingredients, places them
in the grid the way the pattern declares, asserts the result slot's **id and count**, takes it,
and asserts the **inventory counts changed by exactly the recipe's amounts** (each ingredient
down by what it used, the output up by its count). `dielectric_paste` also asserts the
`minecraft:lava_bucket` leaves a `minecraft:bucket` in the grid — the crafting remainder.
Two controls run in the same leg: an off-pattern grid (a stick where the casing belongs) leaves
the result empty, and `SELFTEST=1` expects a wrong output count so the run must fail.

`notes.md` carries the coverage table at the end of this section.

### Results (2026-11-20)

**TDD order as run.** The carry and the recipe leg were written first, then the same
`tools/craft_check.py` was run against the **0.2.1** package: the first two cases reported
`got None xNone` (the paste did not craft) and the third could not find `powah:dielectric_paste`
in the inventory at all; the run ended `EXIT:1`
(`evidence/powah-e2e/e2e-selftest-oldpackage.log`). The 0.2.2 package then made the same script
green, and the whole leg was run four times (main, control, two selftests).

Ladder (every step in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job powah-ladder --port ports/powah --step build` lease;
logs in `ports/_shared/evidence/powah/`):

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **23 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical**, 83,289 B (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.2 … 83289 bytes, SHA-256 9b600446…14bcdf3` (`sdk-check.log`) |
| fits the build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `Powah: / 0b14f8a  made for`, **exit 0** (`sdk-fits.log`) |
| package contents | `unzip -l powah.hmod` | **90 entries**, no `Cargo.lock`, version 0.2.2 |

In-game (launcher build at `0b14f8a` carrying this package; logs in
`ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e -- bash ports/powah/tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| selftest (slice 2's flip) | `SELFTEST=craft …` | `FAIL: … expected 25 powah:dielectric_paste, got powah:dielectric_paste x24` + `EXIT:1` (`e2e-selftest-craft.log`) |
| old package | `craft_check.py` against the 0.2.1 build | every case `got None xNone` + `EXIT:1` (`e2e-selftest-oldpackage.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | `logs=7 ok=7 missing-exit=0 bad-exit=0`, exit 0 |

Quoted from `e2e-launcher.log` (the recipe leg, then the loot leg):

```
3 coal + 2 clay + lava bucket -> 24 dielectric paste -> {"count": 24, "item": "powah:dielectric_paste", "slot": 200}
2 coal + clay + blaze powder -> 16 dielectric paste -> {"count": 16, "item": "powah:dielectric_paste", "slot": 200}
paste + iron bars -> 8 dielectric rods -> {"count": 8, "item": "powah:dielectric_rod", "slot": 200}
paste + iron bars -> 8 horizontal dielectric rods -> {"count": 8, "item": "powah:dielectric_rod_horizontal", "slot": 200}
one horizontal rod -> one vertical rod -> {"count": 1, "item": "powah:dielectric_rod", "slot": 200}
one vertical rod -> one horizontal rod -> {"count": 1, "item": "powah:dielectric_rod_horizontal", "slot": 200}
iron + paste + redstone block -> 4 basic capacitors -> {"count": 4, "item": "powah:capacitor_basic", "slot": 200}
one basic capacitor -> two tiny capacitors -> {"count": 2, "item": "powah:capacitor_basic_tiny", "slot": 200}
3 paste + 2 tiny capacitors + casing + furnace -> a furnator -> {"count": 1, "item": "powah:furnator_starter", "slot": 200}
4 iron + 4 tiny capacitors + casing -> an energy cell -> {"count": 1, "item": "powah:energy_cell_starter", "slot": 200}
6 horizontal rods + 2 nuggets + tiny capacitor -> 12 cables -> {"count": 12, "item": "powah:energy_cable_starter", "slot": 200}
furnator shape with a stick -> {"item": null, "slot": 200}
recipe PASS: the eight material recipes and the three machine recipes craft their declared output id and count,
every ingredient is consumed, the lava bucket leaves its bucket, and the off-pattern grid leaves the result empty
  block(6,100,0) = powah:furnator_starter
  block(7,100,0) = powah:energy_cell_starter
  block(8,100,0) = powah:energy_cable_starter
```

The consumption half is asserted inside `craft_check.py` against `ctl inventory`: each case reads
the counts after `give`, crafts, takes the result into an empty slot, and fails unless every
ingredient fell by exactly the number the pattern used and the output rose by its count; the
paste case also reads the grid back and requires the `minecraft:bucket` remainder there. The
loot leg switches to survival with an iron pickaxe in hand and asserts `drops` contains the
machine.

Package: `powah.hmod`, **83,097 B**, sha256
`9b600446d7457809af1583b3f478881de5b066add4035c0d18ba14ae414bcdf3`.

### Coverage (parity rule, every family the package ships)

| Family (from the jar) | Status |
|---|---|
| `furnator_starter` block + item | **tested** — placed (`place`), given, `info` readout, fuel burned, FE produced (`e2e-launcher.log` phases 1–5) |
| `energy_cell_starter` block + item | **tested** — placed, `set`, full/empty controls, source and sink of a transfer |
| `energy_cable_starter` block + item | **tested** — placed, `no storage, moves up to 500 FE/t`, carries FE, and the no-cable control |
| 3 block loot tables | **tested** — mined in survival, each drops itself (loot leg) |
| `dielectric_paste`, `dielectric_rod`, `dielectric_rod_horizontal`, `dielectric_casing`, `capacitor_basic`, `capacitor_basic_tiny` | **tested** — `ctl give` + `inventory` counts, and each is the output of a crafted recipe |
| 8 material recipes | **tested** — crafted in a real crafting table, output id + count, ingredients consumed, bucket remainder |
| 3 machine recipes | **tested** — same leg; they were dead data in 0.2.1 |
| creative tab (9 items) | **not asserted in game** — no `ctl` command reads a creative tab; `cargo horizon-mod check` validates the tab file and its ids, and the leg proves each id is giveable |
| `powah:capacitor_basic_large` | **not ported** — its consumers (`battery_basic`, `reactor_basic`, the higher capacitor tiers) are not in this slice |
| the six machine tiers, creative cell, Magmator, Thermo, Solar, Reactors, Ender cells/gates, Energizing, Discharger, Hopper, Player Transmitter, batteries, wrench, book, upgrades, GUIs, side config, redstone modes, worldgen | **not ported** — the recorded gaps of slice 1 (items 1–9, 11) |

### Pin findings this slice measured (for the next port)

1. **A menu's slots need `--offscreen`.** With `--headless`, `ctl screen` and `ctl ui` time out on
   an open menu (measured twice: no menu and a crafting table); with `--offscreen` they answer.
   The e2e now launches `--offscreen`.
2. **`ctl interact` refuses a face the player's eye cannot see** (`no face of block … is visible
   from here`): stand ~2.5 blocks away and `ctl look-at-block` first. Standing right beside the
   block puts the eye above it.
3. **Shapeless matching counts grid cells, not items** — `crafting.rs:2013`
   (`matches_all(ctx.context, ingredients, &input.stacks())`) with
   `crafting/shapeless_matching.rs` (`ingredients.len() != stacks.len()` → false). Three coals in
   one cell never match three coal ingredients; one coal in each of three cells does. The port's
   shapeless recipes stay upstream's; the leg places one item per cell. **Worth an upstream
   report**: vanilla's `StackedContents` counts items, so a vanilla-shaped pack behaves
   differently.
4. **A recipe ingredient must be the string form** (`"minecraft:iron_ingot"`,
   `"#minecraft:coals"`); an object (`{"item": …}`) is logged as *"recipes: invalid ingredient
   (unresolved tag or malformed input)"* and the whole recipe is dropped. Other ports carry the
   object form (e.g. `enderstorage`), so their recipes may be silently absent — not this port's
   file to change, recorded here.
5. **Two chat round trips are ~40 ticks apart**, so a two-block assertion can read the source
   before the transfer and the sink after it (measured: source 1000 at t=508, sink 500 at t=513).
   The slice-1 discharge phase now polls the source until the move shows; it was flaky before.

### Deviations slice 2 adds

1. **`capacitor_basic_large` is not carried** (see the coverage table: nothing in this slice uses it).
2. **The paste recipes are matched one-cell-per-item** (finding 3): upstream's shapeless shape is
   kept, but a player stacking three coals in one cell will not match until the pin matches items.
3. **The recipes are written in the 26.3 string form** (finding 4), which is a format, not a
   behaviour difference: the ids, counts, shapes and ingredients are the jar's.

Version: **0.2.1 → 0.2.2** (one patch). The published `[[versions]]` record keeps 0.2.1 and gains
0.2.2 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, verification-before-completion; BeautifulRust/none (no Rust change; `rust-best-practices` loaded and the Rust tree is untouched)

## Slice 3 — the cable network (2026-11-21)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 2 (the notes' own next-slice authority):

> **No cable network.** Upstream's cables share one buffer across a connected
> network and forward an insertion through it in one call. The port moves FE one
> cable hop per tick between *adjacent block entities*, so a run of two or more
> cables does not relay. The e2e's chain is exactly one cable (the slice's brief).

`FINDINGS.md` §Energy Cable reads the same mechanic upstream (`CableTile` /
`CableNet`: walk the connected same-tier cables round-robin and push into each
cable's energy side). So this is a family in the jar, named in the port's gaps.

**Why it fits `machines`:** it is a block-entity relay — the cable's
`BlockEntityPart` tick plus the level's energy capability and `move_energy`, the
same three API pieces slice 1 already declares; no renderer, no GUI, no new item,
no client half. And it is implementable at the pin: `LevelRef::block(host,
pos)?.id(host)` reads a neighbour's block id, which is all a network walk needs.

**Not chosen (and why):** item charging (gap 4) needs `item_energy_buffer` *and*
a chargeable item — Powah's batteries are unported, so nothing in the package can
be charged and the assertion could not fail; the comparator (gap 7) and side
config/redstone (gap 6) are one-hook families; the other six tiers (gap 11) name
no single next machine and need a full content carry first.

### What it ships

No content file changed: the package still carries the same **90 entries** (43
`assets/`, 38 `data/`, 3 `src/`, the 6 root documents). All of slice 3 is Rust.

| File | Change |
|---|---|
| `src/runtime.rs` | The cable planner's loop became a private `plan(sides, transfer, rotation)`; `cable_moves` (six sides) and the new **`network_moves(ports, transfer, rotation)`** both delegate to it. 5 new tests. |
| `src/powah.rs` | `discover_cables` (breadth-first walk over same-tier cable blocks, capped at `NETWORK_MAX_CABLES = 1024`), `relay_network` (gathers the network's non-cable energy sides once per position, plans once, one transaction per move), and `on_level_tick_post` now plans each network once per tick. The single-hop `relay` is gone. |

The machine numbers are unchanged (furnator 20,000 / 80 / 20 FE/t, cell
1,000,000 / 1,000 a side, cable 500 FE/t). The **network spends one cable's 500
FE/t per tick**, not one per cable: the readout's "moves up to 500 FE/t" stays true
for the whole run, and a longer run is not faster. Upstream caps *each* energy
side at a cable's transfer; this port pulls, so the network is one relay and its
budget is one cable's rate. That is a recorded deviation (below), not a defect —
it keeps slice 1's measured number.

### TDD: the red proof, quoted

`network_moves` was declared with a `todo!()` body and the five tests written
against it, then run (`ports/_shared/evidence/powah/slice3-test-red.log`):

```text
---- runtime::tests::a_network_relays_across_the_cables_between_the_source_and_the_sink stdout ----

thread 'runtime::tests::a_network_relays_across_the_cables_between_the_source_and_the_sink' (1797) panicked at src/runtime.rs:209:5:
not yet implemented: the cable network planner
...
test result: FAILED. 23 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

All five failed for the same reason (the planner was not written), and the five
names were:

- `a_network_relays_across_the_cables_between_the_source_and_the_sink`
- `a_network_spends_one_cable_s_rate_not_a_rate_per_cable`
- `one_cable_is_the_same_plan_as_its_six_sides` (the generalization is
  behaviour-preserving; it cross-checks `cable_moves` against `network_moves`)
- `a_network_without_a_giver_or_a_taker_moves_nothing`
- `the_network_rotation_starts_at_another_port`

After the implementation the same command printed **28 passed, 0 failed**
(`slice3-test-green.log`), 23 slice-1/2 tests plus the five.

### In-game (parity rule)

The leg is a **red→green against the previous package**: the 0.2.2 build (made for
`0b14f8a`, kept at `ports/_shared/evidence/powah/powah-0.2.2.hmod`) run with the
new scenario delivers **0 FE** across the three-cable row and fails there:

```text
  source: Energy Cell at 0,100,5: 100000/1000000 FE (comparator 1, t=758)
  sink:   Energy Cell at 4,100,5: 0/1000000 FE (comparator 0, t=624)
  off the network: Energy Cell at 0,103,5: 0/1000000 FE (comparator 0, t=763)
FAIL: a three-cable network did not relay FE into the cell: Energy Cell at 4,100,5: 0/1000000 FE (comparator 0, t=624)
```

(`ports/_shared/evidence/powah-e2e/e2e-slice3-oldpackage.log`, `EXIT:1`.) 0.2.3,
the same scenario (`e2e-launcher.log`, `EXIT:0`):

```text
== slice 3: a run of three cables relays as one network ==
  block(0,100,5) = powah:energy_cell_starter
  block(1,100,5) = powah:energy_cable_starter
  block(2,100,5) = powah:energy_cable_starter
  block(3,100,5) = powah:energy_cable_starter
  block(4,100,5) = powah:energy_cell_starter
  block(0,103,5) = powah:energy_cell_starter
  set(0,100,5) = Energy Cell at 0,100,5: 100000/1000000 FE (comparator 1, t=698)
  source: Energy Cell at 0,100,5: 99500/1000000 FE (comparator 1, t=757)
  sink:   Energy Cell at 4,100,5: 500/1000000 FE (comparator 0, t=747)
  off the network: Energy Cell at 0,103,5: 0/1000000 FE (comparator 0, t=765)
  split: Energy Cell at 0,100,5: 99500/1000000 FE (comparator 1, t=784)
  split: Energy Cell at 4,100,5: 500/1000000 FE (comparator 0, t=794)
```

The three assertions, each able to fail:

1. **the run relays** — the sink holds > 0 (0 on 0.2.2, 0 on `SELFTEST=network`);
2. **nothing is created or lost** — the middle cable is broken so the two cells
   stop trading, and then the two readings must sum to exactly the 100,000 the
   source was set to (99,500 + 500);
3. **the walk is what carries it** — a cell three blocks above the source, reached
   by no cable, stays at 0, so the charge did not teleport.

### Ladder (0.2.3, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease
with `BUILD_LOCAL=1`; logs in `ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **28 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 85,383 B** (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.3 (native-mod, format native-1/0a875da…/e59da90…)`, `85383 bytes, SHA-256 e9583d43…9b3b9c` (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`sdk-fits-nightly.log`) |
| what it uses | `cargo horizon-mod check --uses` | 312 items, made for `0a875da` |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical, no packed `Cargo.lock`, 90 entries |

The `fits` pair matters: `--build 0a875da` is the pin and only prints `made for`
(a tautology); the same package against the previous pin `0b14f8a` says **fits**,
i.e. every one of its 312 API items still resolves there.

In-game legs (launcher build `native-0a875da-e8d17b7a402b`, instance
`powah-e2e`; logs in `ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e -- bash tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| selftest (slice 3's flip) | `SELFTEST=network …` | `FAIL: SELFTEST flipped: the three-cable network relayed FE, as it should` + `EXIT:1` (`e2e-selftest-network.log`) |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| the previous package | the 0.2.2 build + the same scenario | `EXIT:1` at the network assertion (`e2e-slice3-oldpackage.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | `logs=9 ok=9 missing-exit=0 bad-exit=0`, exit 0 |

The main log also carries the in-script control leg (the same Nightly without
powah knows neither block id and neither item).

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| cable **network** (`CableNet`: connected same-tier cables forward one insertion) | **tested** — a source cell, three cables and a sink cell in one row: the charge crosses the run (0 on 0.2.2), the split cells sum to the 100,000 set, and a cell off the network stays 0; the planner's five rules are unit-tested |
| single-cable relay (slice 1) | **tested** — unchanged: `one_cable_is_the_same_plan_as_its_six_sides` compares the new network plan with the six-side plan, and the slice-1 phases still pass in the same run |
| every other family | as slice 2's table: ported and tested, or `not ported` with the gap that names it |

### Deviations slice 3 adds

1. **One cable's rate per network per tick** (500 FE/t), not one per cable: the
   port pulls, so the network is one relay; upstream pushes into each energy side
   at up to a cable's transfer. A ten-cable run therefore carries the same 500
   FE/t as a one-cable run.
2. **The walk is capped at 1,024 cables** per network (`NETWORK_MAX_CABLES`).
   Upstream's `CableNet` is unbounded; the cap bounds one tick's work and is far
   past any pack's run.
3. **A block touching two cables of one network is one port** (deduplicated by
   position), so a cell flanked by two cables of the same network is filled once
   per plan rather than twice. With deviation 1's single budget this changes no
   observable rate.
4. **Per-tier isolation is by block id**: only `powah:energy_cable_starter` exists
   in this slice, so the walk treats every cable it finds as the same network. When
   the other six tiers arrive, the membership test becomes same-tier.
5. The slice-1 gaps that remain are unchanged: no GUI, no item charging, no side
   config/redstone modes, no comparator hook, breaking a machine loses its FE, the
   other six tiers and ~15 machines unported, `/powah set` is a harness control.

**Upstream defects fixed:** none — the single-hop relay was the *port's*
limitation (slice 1's recorded scope), not a defect in Powah 6.2.10, so the
upstream-defect procedure did not apply.

### Pin findings this slice measured (for the next port)

1. **The pin moved mid-slice** (`0b14f8a` → `0a875da`, mixins unchanged). Every
   number above is at `0a875da`; the package declares it, and the launcher's
   `check_pins` refuses a package pinned elsewhere. `fits --build <the previous
   pin>` is the run that proves something.
2. **A launcher instance that already holds an older install of the port refuses
   the new package.** `native-build --instance e2e` (an instance shared with a
   previous `powah` install from `0b14f8a`) answered *"powah's package was made by
   an older tool and doesn't say what it uses, so it works on the Nov 20 update
   only"*, although the same folder carries a valid `uses.json` (312 entries,
   `made_for` = the pin) and `cargo horizon-mod fits` on the folder is green. With
   a dedicated instance (`HORIZON_E2E_INSTANCE=powah-e2e`) the identical folder
   built in 7 s. **Give each port its own instance**, which is also what stops two
   agents rebuilding each other's instance.
3. **Two `info` reads are ~20–40 ticks apart, and adjacent cells swap energy every
   tick**, so a sum of two live readings is *not* the total: the first version of
   the conservation assertion read 100,500 (source at `t=757`, sink at `t=733`) on
   a balanced relay. Freeze the transfer (break a cable) before summing.
4. **`BUILD_LOCAL=1` for evidence integrity.** A forwarded step runs on the far
   box and then `remote-slot.sh` pulls `ports/_shared/evidence/<mod>/` back, which
   **overwrote a fresh local `sdk-test.log` with the far box's older file of the
   same name** (mtime and content from the previous day). Local runs keep each log
   an artefact of the run it names.

Version: **0.2.2 → 0.2.3** (one patch), pin `0b14f8a` → `0a875da`. The published
`[[versions]]` record keeps 0.2.2 and gains 0.2.3 at publish time — this port does
not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 4 — the energy cell ladder (2026-11-21)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 11 — the notes' own next-slice authority:

> **11. The other 6 tiers, the creative cell and ~15 machines** are unported.

and `FINDINGS.md` §"Tier numbering", whose derived table names the numbers this slice
carries:

> cell 1M/4M/10M/40M/100M/400M/2G

**Chosen:** the six remaining **energy cell** tiers (`basic`, `hardened`, `blazing`,
`niotic`, `spirited`, `nitro`) as storage blocks with their own capacity and per-side
transfer — the ladder the starter cell's row already begins. **Why it fits
`machines`:** a cell is the tier's shape exactly — a declared block energy capability
(`Registrar::block_energy`) over the saved int `energy`, one block entity type and an
idle tick, and the `push_out` the starter cell already runs; no new hook family, no
renderer, no GUI, no item behaviour. Its rule is the most directly assertable in the
package: `/powah set 2000000000` against each tier must clamp at **that tier's**
capacity, so one command reads back a different exact number per tier (a
before → after `ctl` readout that the 0.2.3 package fails outright: it does not know
the ids).

**Not chosen (and why):** the reactors (`FINDINGS.md` names them) need
`powah:uraninite`, whose every source is the **unported Energizing Orb** or worldgen,
so no reactor recipe could be crafted; the Magmator/Thermo generators need a fluid
tank (`fluid_insert` cannot fill a mod BE tank at this pin, measured) and the Solar
panel needs sky visibility; the other six **furnator/cable** tiers are the same shape
as this slice but their rule is a rate into a buffer (a bound), not an exact readout.

**Recipes:** the jar's `energy_cell_basic` recipe is craftable from carried items
(`4 × capacitor_basic`, `dielectric_casing`, `4 × iron_ingot`), so it is carried and
crafted in game. The other five cell recipes take `powah:steel_energized` /
`powah:crystal_blazing|niotic|spirited|nitro`, which are only made by the unported
**Energizing Orb**; those recipe files are **not carried** (a dead recipe is worse
than a named gap) and the coverage table records `blocked(energizing-orb)` with the
exact missing ids. Every tier block itself is placed, set and read in game.

### What it ships

| Family | Files |
|---|---|
| blocks (6) | `data/powah/horizon/block/energy_cell_{basic,hardened,blazing,niotic,spirited,nitro}.json` (the starter's patched def: `minecraft:iron_block`, 2.0/20.0, metal, `requires_tool`, `occludes: false`, stack 1) |
| blockstates (6) | `assets/powah/blockstates/energy_cell_<tier>.json`, carried |
| block models (6) | `assets/powah/models/block/energy_cell_<tier>.json` (parent `powah:block/energy_cell`, the tier's texture) |
| item models (6) + item defs (6) | `assets/powah/models/item/…`, `assets/powah/items/…`, carried |
| textures (6) | `assets/powah/textures/block/energy_cell_<tier>.png`, byte-for-byte |
| loot (6) | `data/powah/loot_table/blocks/energy_cell_<tier>.json` (the jar's 26.3 shape) |
| tags | the six join `minecraft:mineable/pickaxe` and `needs_iron_tool` |
| creative tab | the six join `powah:horizon/creative_tab/machines.json` |
| recipe + advancement (1 + 1) | `energy_cell_basic` only (see above) |

Rust, all in the port's own tree:

| File | Change |
|---|---|
| `src/runtime.rs` | `CellTier { capacity, transfer }`, `CELL_TIER_COUNT`, `cell_tier(index)` derived from upstream's `baseScaling` (`× 1_000_000` capacity, `× 1_000` transfer); 2 new tests |
| `src/powah.rs` | `CELLS` (the seven ids + type names), `cell_of(id)`, `Kind::Cell(CellTier)`, `cell_battery(tier)`, per-tier `battery_of`/readout/set clamp/push cap; 2 new tests |
| `src/lib.rs` | one `block_energy` declaration and one idle block entity type per cell tier (7 buffers, 9 types in total) |

The readout gained the tier's transfer so the in-game leg can read it back:
`Energy Cell at x,y,z: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=739)`.
The starter's line keeps its prefix (every 0.1.0/0.2.x assertion still matches) and
the unit test that pins the exact line was updated.

### TDD: the red proof, quoted

`cell_tier` and `cell_of` were declared with `todo!()` and the tests written against
them, then run (`ports/_shared/evidence/powah/slice4-test-red.log`):

```text
thread 'powah::tests::every_energy_cell_tier_is_one_of_the_ladder' panicked at src/powah.rs:74:5:
not yet implemented: the energy cell block ids

thread 'runtime::tests::every_energy_cell_tier_scales_upstreams_battery_numbers' panicked at src/runtime.rs:44:5:
not yet implemented: the energy cell tier ladder

thread 'runtime::tests::the_starter_cell_keeps_slice_1_s_numbers' panicked at src/runtime.rs:44:5:
not yet implemented: the energy cell tier ladder

test result: FAILED. 28 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

After the implementation: **32 passed, 0 failed**
(`slice4-test-green.log`; the same 32 in `sdk-test.log`). The fourth new test,
`every_cell_tier_declares_its_own_buffer_and_nothing_else_does`, was added with the
wiring after that run (it pins `battery_of` to the same table); the wiring's red
proof is the in-game one below.

The in-game red is the same scenario against the **0.2.3** package
(`ports/_shared/evidence/powah/powah-0.2.3.hmod`, run on a launcher build of that
package's own tree, `evidence/powah-e2e/e2e-slice4-oldpackage.log`):

```text
== slice 4: every energy cell tier holds its own capacity ==
FAIL: expected powah:energy_cell_basic at -4,100,12, got 'minecraft:air'
```
(`Unknown block type 'powah:energy_cell_basic'` in the game log; `EXIT:1`.)

### Ladder (0.2.4, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease with
`BUILD_LOCAL=1`; logs in `ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **32 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 119,200 B** (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.4 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `119200 bytes, SHA-256 663465ee8b93e5f8be371537f6515bd7252651e462a57f477faedd82ffcd3e34, unsigned` (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`sdk-fits-nightly.log`) |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical (`e49a2edda151`), no packed `Cargo.lock`, **134 entries** |
| carried-data dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: 33 recipe/loot files, every one in the 26.3 dialect, every `requires_tool` block in a mineable tag |

In-game legs (launcher build `native-0a875da-f6fbf1a8142b`, instance `powah-e2e`,
`Compiling powah (1 of 1)` in `evidence/powah-e2e/launcher-build-0.2.4.log`; logs in
`ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e -- bash tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (inside the main run; same Nightly, no package) | `CONTROL_GAME` | `setblock` → `Unknown block type 'powah:energy_cell_nitro'`, `give` → `Unknown item 'powah:energy_cell_starter'`, block stays air |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| selftest (this slice's flip) | `SELFTEST=cells …` | `FAIL: expected powah:energy_cell_basic to hold 1000000/1000000 FE, got 'Energy Cell at -4,100,12: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=766)'` + `EXIT:1` (`e2e-selftest-cells.log`) |
| the previous package | 0.2.3 + this scenario | `EXIT:1` at the first new tier (`e2e-slice4-oldpackage.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | see below |

Quoted from `e2e-launcher.log` (one command per tier; every tier clamps at its own
capacity and names its own transfer):

```text
== slice 4: every energy cell tier holds its own capacity ==
  block(-4,100,12) = powah:energy_cell_basic
  set(-4,100,12) = Energy Cell at -4,100,12: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=714)
  block(-2,100,12) = powah:energy_cell_hardened
  set(-2,100,12) = Energy Cell at -2,100,12: 10000000/10000000 FE (comparator 15, 10000 FE/t a side, t=728)
  block(0,100,12) = powah:energy_cell_blazing
  set(0,100,12) = Energy Cell at 0,100,12: 40000000/40000000 FE (comparator 15, 40000 FE/t a side, t=744)
  block(2,100,12) = powah:energy_cell_niotic
  set(2,100,12) = Energy Cell at 2,100,12: 100000000/100000000 FE (comparator 15, 100000 FE/t a side, t=762)
  block(4,100,12) = powah:energy_cell_spirited
  set(4,100,12) = Energy Cell at 4,100,12: 400000000/400000000 FE (comparator 15, 400000 FE/t a side, t=778)
  block(6,100,12) = powah:energy_cell_nitro
  set(6,100,12) = Energy Cell at 6,100,12: 2000000000/2000000000 FE (comparator 15, 2000000 FE/t a side, t=794)
  block(-6,100,12) = powah:energy_cell_starter
  set(-6,100,12) = Energy Cell at -6,100,12: 1000000/1000000 FE (comparator 15, 1000 FE/t a side, t=810)
== slice 4: two adjacent cells swap and create nothing ==
  pair source: Energy Cell at 16,100,16: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=892)
  pair sink:   Energy Cell at 16,100,17: 0/1000000 FE (comparator 0, 1000 FE/t a side, t=899)
  {"scenario":"loot","block":"minecraft:air","drops":{"powah:energy_cell_basic":1}}
  {"scenario":"loot","block":"minecraft:air","drops":{"powah:energy_cell_nitro":1}}
```

### What the in-game leg can and cannot assert about the transfer

The **capacity** is asserted behaviourally and exactly: `2000000000` is above every
tier's capacity, so the clamp is the tier's number, and it differs per tier.

The **transfer** is asserted as the declaration the readout reads back, and pinned by
`every_cell_tier_declares_its_own_buffer_and_nothing_else_does` (the same `cell_tier`
row feeds `Registrar::block_energy`). A *net* flow between two cells is not
observable at this pin, and this is upstream's behaviour rather than a port gap: a
cell gives `min(transfer, stored)` and the neighbour accepts only
`min(the neighbour's own insert cap, room)`, and both push every tick, so any adjacent
pair swaps and nets zero — measured both ways (basic source → starter sink and
starter → starter), and the pair phase asserts exactly that (the sink never climbs
past its own 1,000 insert cap beside a 4,000/t source, and the source never loses more
than the same cap, so the pair swaps rather than creating FE).

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| `energy_cell_basic` block + item | **tested** — placed, `/powah set 2000000000` clamps at 4,000,000, readout names 4,000/t; mined in survival, drops itself |
| `energy_cell_hardened` | **tested** — same legs; clamps 10,000,000, 10,000/t |
| `energy_cell_blazing` | **tested** — clamps 40,000,000, 40,000/t |
| `energy_cell_niotic` | **tested** — clamps 100,000,000, 100,000/t |
| `energy_cell_spirited` | **tested** — clamps 400,000,000, 400,000/t |
| `energy_cell_nitro` | **tested** — clamps 2,000,000,000, 2,000,000/t (the largest value the integer command accepts) |
| `energy_cell_starter` (contrast) | **tested** — the same command clamps at 1,000,000, 1,000/t; slice 1's legs unchanged |
| `energy_cell_basic` recipe | **tested** — crafted in a real table (`craft_check.py`): id + count, every ingredient consumed |
| `energy_cell_{hardened,blazing,niotic,spirited,nitro}` recipes | **blocked(energizing-orb)** — they need `powah:steel_energized` / `powah:crystal_{blazing,niotic,spirited,nitro}`, all made by the unported Energizing Orb; the files are not carried |
| the six cells' loot tables | **tested** — mined in survival, each drops itself (6 rows in the loot leg) |
| creative tab (15 items now) | **not asserted in game** — no `ctl` command reads a creative tab; `cargo horizon-mod check` validates the tab, and the leg proves each id is giveable/placeable |
| `energy_cell_creative` | **not ported** — a source, not a storage tier (gap 11) |
| the other tiers/machines (furnator, cable, generators, reactors, Energizing, batteries, GUIs, side config, redstone modes, item charging, worldgen) | **not ported** — slice 1's recorded gaps 1–9, 11; `## Next slice` names the Energizing Orb as the one that unblocks the five recipes above |

### Deviations slice 4 adds

1. **Five of the six cell recipes are not carried** (see above): the blocks exist and
   are provable, but in survival only `energy_cell_basic` can be reached until the
   Energizing Orb ships. Recorded as `blocked(energizing-orb)`, not hidden.
2. **The readout gained the tier's transfer** (`… FE/t a side`) so the in-game leg can
   read the declaration back. The starter's line keeps its prefix.
3. **The side config, redstone modes, comparator hook, GUI and item charging** remain
   slice 1's gaps for every tier, the starter included.

**Upstream defects fixed:** none — the tier numbers are upstream's
(`DefaultEnergies`), and the swap between adjacent cells is upstream's own
`Transfer.ALL` behaviour, documented (slice 1 deviation 3), not a defect. The only
defect found while measuring was in **this port's harness**, not the mod: three
assertions read a sink once before the relay had moved (slice 1's discharge phase,
slice 3's network sink, and `set_energy`'s exact-equality poll against a cell that a
cable drains every tick) — all three are now poll-based (`set_floor`, sink re-reads),
and the two flaky runs that exposed them are kept in
`evidence/powah-e2e/` history only through the logs they overwrote.

### Pin findings this slice measured (for the next port)

1. **A cell pair always nets zero.** A cell gives `min(transfer, stored)` and accepts
   only `min(its own insert cap, room)`, and both push; a higher-tier source's extra
   transfer is truncated by the lower-tier sink's insert cap and returned the same
   tick. Any assertion of a tier's transfer must therefore be a declaration test (the
   readout and a unit test on `battery_of`), never a net flow between two cells.
2. **`/powah set` on a source that feeds something cannot be polled for equality** —
   the drain starts the same tick (measured 99,500 by the next chat round trip). A
   `set_floor` (≥ a floor) poll is the right shape; the conservation assertion carries
   the exact total.
3. **A sink read can precede the move the source poll waited for** by one round trip
   (measured: sink 0 at t=552, source 500 at t=557). Poll the sink too.
4. `/powah info`'s cell line now carries the declared transfer, so a harness can
   assert a tier's declaration without a second command.

Version: **0.2.3 → 0.2.4** (one patch). The published `[[versions]]` record keeps
0.2.3 and gains 0.2.4 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

### Coverage table in the checker's shape (`check-coverage.py`)

| Family | Status | Evidence |
|---|---|---|
| `block` | tested | every block placed and read back, from the three slice-1 machines to the seven cell tiers and their loot |
| `item` | tested | every item given and counted; the machines' items placed |
| `block_entity` | tested | the furnator burns and its buffer fills; a cell holds its tier's capacity; the cable relays across a run (energy only moves through the block entities' ticks) |
| `loot_table` | tested | every machine and cell block mined in survival in the loot leg and asserted to drop itself |
| `recipe:minecraft:crafting_shaped` | tested | `craft_check.py` crafts each shaped recipe in a real table: output id + count, ingredients consumed |
| `recipe:minecraft:crafting_shapeless` | tested | same leg; the paste's lava bucket leaves its bucket |
| `tag:block` | tested | the cable's `powah:energy_cable_connects` is the visual-arms tag (the relay is the mechanic and the three-cable run is asserted); the pickaxe/needs-iron tags are what makes the loot leg's drops real |
