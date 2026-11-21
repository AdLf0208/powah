# Powah — content (slices 1 and 2)

The carried content of the starter Furnator, Energy Cell and Energy Cable (slice 1) and of the
dielectric/capacitor crafting family (slice 2). Since 0.2.0 the package root **is** the mod root
(`horizon-mod.toml` + `assets/` + `data/` beside `src/`), and `powah.hmod` is the one built
artifact.

**Slice 3 (the cable network, 0.2.3) changed no file here**: it is Rust only.
**Slice 4 (the energy cell ladder, 0.2.4)** adds the six higher tiers' blocks, blockstates, block and
item models, item definitions, textures, loot tables, the pickaxe/needs-iron tag entries, their
creative-tab rows, and the one craftable recipe (`energy_cell_basic`) with its advancement — 44 new
entries, for **134** in the package. The five higher cells' recipes are deliberately absent: their
inputs come from the unported Energizing Orb (see `notes.md`, `blocked(energizing-orb)`).

Every file comes from the raw carry of the ATM10 jar `Powah-6.2.10.jar`
(Crash Assistant `modlist.json`; Modrinth SHA-1
`f134ab3e0ace3793abf93129e318eb817898a4e1`, 2,737,991 B) in
`ports/_shared/carry-work/powah/carry/`, produced by `horizon-port-pack check`
at the campaign pin. `tools/assemble_content.py` (in the port root) reproduces
this folder and applies the patches below; **no asset or model is invented**.

| Path | What |
|---|---|
| `data/powah/horizon/block/furnator_starter.json` | **Patched** from the carry's `minecraft:carved_pumpkin` guess to `minecraft:furnace` (the block has a horizontal `facing`); hardness 2.0, resistance 20.0, `minecraft:iron_block` sounds, tool required, `occludes: false` (upstream `metalNoSolid`), stack 1 |
| `data/powah/horizon/block/energy_cell_starter.json` | **Patched** from the carry's `minecraft:stone` to `minecraft:iron_block` (no states) with the same numbers |
| `data/powah/horizon/block/energy_cable_starter.json` | **New**: the carry left the cable out (`no template for properties [down,east,north,south,up,west]`). `minecraft:chorus_plant` + `connects_to: "#powah:energy_cable_connects"` + `thickness: 4.0`, empty collision (upstream `noCollission`) |
| `assets/powah/blockstates/furnator_starter.json` | **Patched**: dropped `lollipop:block/empty` (another mod's model) for `facing=up/down`; 4 facings × `lit` (never set; upstream's `burning` is a sync flag, not a state) |
| `assets/powah/models/block/energy_cell.json` | **Patched**: parent `block` (1.21.1's `models/block.json`) → `block/block`, the only one 26.3 ships |
| `data/minecraft/tags/block/mineable/pickaxe.json`, `needs_iron_tool.json` | **Patched** to the three ids that exist (the carry lists all 92) |
| `data/powah/tags/block/energy_cable_connects.json` | **New**: the cable's neighbours for its visual arms (upstream joins any FE block; a content tag can only name blocks) |
| `data/powah/recipe/crafting/cable_starter.json` + `data/powah/advancement/recipes/misc/crafting/cable_starter.json` | **From the jar**: the carry skipped them with the block. 12 cables per recipe |
| `data/powah/horizon/creative_tab/machines.json` | **New**: upstream's tab is code; one tab with the three items (title key `itemGroup.powah.tab` from the jar's lang) |
| blockstates, models, `items/`, textures, lang, loot tables, two recipes/advancements | The carry's 26.3 output, byte-for-byte |

The three recipes need `powah:capacitor_basic_tiny`, `powah:dielectric_casing`,
`powah:dielectric_paste` and `powah:dielectric_rod_horizontal`. **Slice 2 carries them** (below),
so all three craft.

## Slice 2 — the dielectric/capacitor family (2026-11-20)

Six plain items, carried from the same jar and declared with the same shapes the three machines
use:

| Family | Files |
|---|---|
| items (6) | `data/powah/horizon/item/{dielectric_paste,dielectric_rod,dielectric_rod_horizontal,dielectric_casing,capacitor_basic,capacitor_basic_tiny}.json`, each `{"template": "minecraft:stick"}` (upstream's plain 64-stack items) |
| item definitions (6) | `assets/powah/items/<id>.json`, the 26.3 item-model definitions the three existing items already use |
| models (6) | `assets/powah/models/item/<id>.json`, byte-for-byte from the jar (`dielectric_casing` is its Blockbench 3D model on `block/block`) |
| textures (6) | `assets/powah/textures/item/<id>.png`, byte-for-byte |
| recipes (8) | `data/powah/recipe/crafting/{dielectric_paste,dielectric_paste_2,dielectric_rod,dielectric_rod_2,dielectric_rod_h,dielectric_rod_h_2,capacitor_basic,capacitor_basic_tiny}.json`, from the jar's recipes in the 26.3 shape |
| advancements (8) | the jar's recipe-unlock files under `data/powah/advancement/recipes/misc/crafting/` |
| creative tab | the six items join `data/powah/horizon/creative_tab/machines.json` (upstream has one Powah tab) |
| lang | unchanged: the `item.powah.*` names were already carried |

Two patches, both from the pin's recipe loader (proved in game, see `notes.md`):

1. **Ingredients are the string form** (`"minecraft:iron_ingot"`, `"#minecraft:coals"`), not the
   object form (`{"item": …}`). The loader rejects an object with *"recipes: invalid ingredient
   (unresolved tag or malformed input)"* and drops the whole file, so a carried 1.21.1 recipe
   written that way would never register.
2. **`dielectric_paste` and `dielectric_paste_2` are shapeless with repeated ingredients**, and
   this pin matches a shapeless recipe's ingredients against the **grid cells**, one stack per
   cell (`crafting.rs` `input.stacks()`, `crafting/shapeless_matching.rs`
   `ingredients.len() != stacks.len()`): a player who puts three coals in one cell matches
   nothing. The recipes stay upstream's (shapeless); the in-game leg places one item per cell,
   and the deviation is recorded in `notes.md` as a pin finding, not a port change.

## Slice 4 — the energy cell tiers (2026-11-21)

Six blocks and their items, carried from the same jar with the starter cell's patched block
definition (upstream's `AbstractEnergyBlock`: 2.0/20.0, metal sounds, `occludes: false`, stack 1):

| Family | Files |
|---|---|
| blocks (6) | `data/powah/horizon/block/energy_cell_{basic,hardened,blazing,niotic,spirited,nitro}.json` |
| blockstates / block models / item models / item definitions (6 each) | `assets/powah/…` under `blockstates/`, `models/block/`, `models/item/`, `items/` |
| textures (6) | `assets/powah/textures/block/energy_cell_<tier>.png` |
| loot tables (6) | `data/powah/loot_table/blocks/energy_cell_<tier>.json` |
| tags | the six join `minecraft:mineable/pickaxe` and `minecraft:needs_iron_tool` |
| creative tab | the six join `data/powah/horizon/creative_tab/machines.json` |
| recipe + advancement | `data/powah/recipe/crafting/energy_cell_basic.json` only (`ici/cdc/ici` with `capacitor_basic`, `dielectric_casing`, iron) |

The five other recipes need `powah:steel_energized` / `powah:crystal_*`, which only the Energizing Orb
makes; they are not carried and are recorded `blocked(energizing-orb)` in `notes.md`.

## Slice 5 — the Energizing Orb, the rod ladder and the materials (2026-11-21)

The gate slice 4 hit: the orb makes `powah:steel_energized` and the four crystals, and without
it five of six cell recipes and the whole capacitor ladder above `capacitor_basic` are
unreachable.

| Family | Files |
|---|---|
| blocks (9) | `data/powah/horizon/block/{energizing_orb,energizing_rod_starter,basic,hardened,blazing,niotic,spirited,nitro,blazing_crystal_block}.json` — the orb and the rods declare the six-value `facing` state upstream's `Facing.ALL` gives them (`minecraft:iron_block` + `state_properties`, default `down`, `occludes: false`, stack 1); the material block is `minecraft:stone` with upstream's 2.0/20.0 metal |
| blockstates (9) | `assets/powah/blockstates/<block>.json`, carried (six `facing` variants each, from the jar) |
| block models (10) | `assets/powah/models/block/<block>.json` plus the rods' shared `energizing_rod.json` parent |
| item models (20) + item definitions (20) | `assets/powah/models/item/…`, `assets/powah/items/…` for the nine blocks and the eleven plain items |
| item definitions (11) | `data/powah/horizon/item/<item>.json` — `minecraft:stick` (64-stack), as slice 2's |
| textures (23) | the orb's three, the rod's shared one and seven gems, the material block, the five materials (three animated, `.mcmeta` carried) and the six capacitors |
| loot (9) | `data/powah/loot_table/blocks/<block>.json` (the jar's 26.3 shape) |
| recipes (21) + advancements (21) | the orb, seven rods, `capacitor_basic_large`, five capacitor tiers, the five previously blocked cells, `blazing_crystal_block` and its 9-back shapeless |
| tags | the nine join `minecraft:mineable/pickaxe`; the jar's `powah:energizing_rods` block and item tags are carried |
| creative tab | the twenty new items join `powah:horizon/creative_tab/machines.json` |
| lang | unchanged: the carry's `en_us.json` already has every key |

**Not carried:** the jar's `data/powah/recipe/energizing/*.json` — `powah:energizing` is a
custom recipe type the pin has no loader for, so the six rows live in `src/runtime.rs` as a
table (`energizing()`), matched per filled slot. `blazing_crystal_2`'s ingredient is
`c:rods/blaze`, which has no 26.3 equivalent, so it is carried as `minecraft:blaze_rod`
(the tag's vanilla member) instead.

Two patches, both from the pin's loader and recorded in `notes.md`:

1. **Ingredients are the string form** (`"minecraft:quartz_block"`), not `{"item": …}`;
2. **NeoForge `c:` tags become the vanilla member they resolve to** — 26.3 has no `c:` tags:
   `c:glass_blocks` → `minecraft:glass`, `c:storage_blocks/quartz` → `minecraft:quartz_block`.
   The same replacement is applied in the recipe-unlock advancements, or a recipe would never
   unlock.

`tools/carry_slice5.py` (in the port root) reproduces this slice's 160 files from the raw carry
and `--check` verifies the tree still matches; **no asset or model is invented**.

## Where this content lives now (0.2.0)

Since the SDK migration the carried tree **is the package root**: `assets/powah/` and
`data/powah/` sit beside `src/`, and `horizon-mod.toml` at the root carries the identity plus
`[native]`. There is no `content/` folder and no separate content `.hmod` any more — one
`powah.hmod` (68,403 B, SHA-256 `11824c7c…455d`, 50 entries) ships this content **with** the
code. The 41 carried files were moved with their paths, sizes and SHA-256 unchanged
(`../_shared/evidence/powah/carried-content-sha256.txt`).

The build of the 0.1.0 content-only archive, kept as history:

Built with (inside a build slot, from this folder):

```sh
CARGO_TARGET_DIR=<slot target>/cargo-horizon-mod cargo run --locked \
  --manifest-path ../../_shared/pins/horizon-1d63e0a/crates/cargo-horizon-mod/Cargo.toml \
  -- horizon-mod build
```

then copied beside the folder as `content/powah.hmod` (39,262 B; SHA-256
`7d344174…`; 42 entries) — retired to
`../_shared/evidence/powah/retired-hnmod-0.1.0/content-old/`.
