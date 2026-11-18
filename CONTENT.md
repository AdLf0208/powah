# Powah — content (slice 1)

The carried content of the starter Furnator, Energy Cell and Energy Cable. Since 0.2.0 the
package root **is** the mod root (`horizon-mod.toml` + `assets/` + `data/` beside `src/`), and
`powah.hmod` is the one built artifact.

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
`powah:dielectric_paste` and `powah:dielectric_rod_horizontal`, which are later
slices — they ship but do not craft yet.

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
