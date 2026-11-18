# Powah — upstream analysis (slice 1: Furnator, Energy Cell, Cable)

Ported from **Powah 6.2.10** (`Technici4n/Powah`, **LGPL-3.0-only**, `mods.toml`
`license="LGPLv3"`), tag `v6.2.10` = commit
`a5a8f25b5bafb58b694dc44ef0e4c0f14d9e4c11`. Jar provenance (exact ATM10 file,
hashes, fetch path): [`ports/_shared/carry-work/powah/SOURCE.md`](../../_shared/carry-work/powah/SOURCE.md).

Analysis only: behaviour, defaults, event flow and edge cases were read from the
public source; no Java was copied or translated. This is a clean-room Rust
rewrite.

## Files read

| Area | Files |
|---|---|
| blocks / tiers | `block/Blcks.java`, `block/Tier.java`, `block/Tiles.java`, `lib/block/Properties.java` |
| furnator | `block/furnator/FurnatorBlock.java`, `block/furnator/FurnatorTile.java` |
| energy cell | `block/energycell/EnergyCellBlock.java`, `block/energycell/EnergyCellTile.java` |
| cable | `block/cable/CableBlock.java`, `block/cable/CableTile.java`, `block/cable/CableNet.java` |
| machine core | `lib/block/AbstractEnergyStorage.java`, `lib/block/AbstractEnergyProvider.java`, `lib/block/AbstractTileEntity.java` |
| energy model | `lib/logistics/energy/Energy.java`, `lib/logistics/energy/SideConfig.java`, `lib/logistics/Transfer.java`, `lib/logistics/Redstone.java` |
| config | `config/v2/PowahConfig.java`, `config/v2/DefaultEnergies.java`, `config/v2/types/{GeneratorConfig,CableConfig,EnergyConfig}.java`, `config/v2/values/TieredEnergyValues.java` |
| content | `assets/powah/blockstates/{furnator,energy_cell,energy_cable}_starter.json`, `models/block/{furnator,energy_cell,cable,cable_multipart}.json`, `data/powah/recipe/crafting/*` |

## Tier numbering (STARTER tier, the one in this slice)

`DefaultEnergies` derives every value from two scalings; a tier's value is the
base scaling at that tier's index (0 = STARTER):

```
baseScaling     = 1, 4, 10, 40, 100, 400, 2000
baseSlowScaling = 1, 3, 5,  10,  20,  40,  100
activeProduction   = baseScaling * 20       -> 20, 80, 200, 800, 2000, 8000, 40000 FE/t
reactorProduction  = baseScaling * 250      -> 250, 1000, 2500, 10000, 25000, 100000, 500000
generatorCapacity  = activeProduction * 1000
generatorTransfer  = activeProduction * 4
batteryCapacity    = reactorProduction * 4000
batteryTransfer    = reactorProduction * 2 * 2
cableTransfer      = reactorProduction * 2
energyPerFuelTick  = 30                      (general.energy_per_fuel_tick)
```

STARTER (slice 1): **furnator** capacity **20,000 FE**, transfer **80 FE/t**
per side, generation **20 FE/t**; **energy cell** capacity **1,000,000 FE**,
transfer **1,000 FE/t** per side; **energy cable** transfer **500 FE/t**, no
storage. Full set of tiers for later slices: capacities 20k/80k/200k/800k/2M/8M/40M,
cell 1M/4M/10M/40M/100M/400M/2G, cable 500/2000/5000/20000/50000/200000/1M,
generation 20/80/200/800/2000/8000/40000.

## Behaviour

### Furnator (`FurnatorTile.postTick`)

1. **Refuel** only when the carbon buffer is *empty* and the block is not
   redstone-disabled (`checkRedstone()`, default `Redstone.IGNORE` = always on),
   and the input slot has an item whose `getBurnTime(RecipeType.SMELTING) > 0`:
   `carbon = burnTime * energy_per_fuel_tick` (30), then consume one item
   (`getCraftingRemainingItem` replaces it when the stack runs out — a lava
   bucket leaves a bucket). The slot is limited to 64. The buffer's room is
   **not** consulted: a full furnator still turns the next item into carbon and
   burns it later.
2. **Produce** only while carbon is non-empty **and the buffer has room**:
   `toProduce = min(space, min(generation, carbon))`; `energy += toProduce`,
   `carbon -= toProduce`, and carbon is zeroed when it reaches 0.
3. `burning` is a *client sync flag* (particles/sound), true on any tick that
   produced energy — **not** a block state; the model never changes.
4. `postTick` returns 10 when it charged an item or pushed energy out, −1
   otherwise; that is the ticker's reschedule hint, not gameplay.
5. The block has two slots: 0 for chargeable items (`Energy.chargeable`), 1 for
   fuel. `canInsert` allows fuel in 1 and chargeables in 0; everything may be
   taken out. `keepEnergy()` is true, so the block item carries the buffer.
6. Every tick it also pushes `min(transfer, stored)` into **each** side whose
   neighbour is an energy capability (`extractFromSides`), and charges the items
   in slots 0..1 (`chargeItems(1)`).

Consequence worth keeping: one coal (1600 ticks) gives `1600*30 = 48,000`
carbon, which at 20 FE/t lasts **2400 ticks** and yields **48,000 FE**; a full
buffer pauses production *without* draining carbon, so no fuel is wasted, and a
partly-charged buffer resumes from the same carbon.

### Energy Cell (`EnergyCellTile`)

- Pure storage: `AbstractEnergyStorage` with capacity 1,000,000 and transfer
  1,000 per side (STARTER). No generation; `postTick` only pushes to neighbours
  and charges items.
- All six sides `Transfer.ALL` by default (insert and extract), gated by
  `checkRedstone()` (default IGNORE).
- `keepEnergy() = !isCreative()`; a creative cell is a source (transfer type
  EXTRACT, never drains). Creative tier is out of this slice.
- Two item slots (charging), slot limit 1.

### Energy Cable (`CableTile`, `CableNet`)

- **No storage**: `getEnergyCapacity() = 0`, `keepStorable() = false`. Nothing is
  saved.
- `receiveEnergy(maxReceive, simulate, direction)` forwards the insertion into
  the **network** in one call: it walks the connected same-tier cables
  (round-robin start index, incremented per insertion), and for each cable's
  energy side (a neighbour that is not a cable and has an energy capability)
  pushes `min(remaining, cable.maxExtract)` into it, skipping the side the
  energy came from. The source only loses what an acceptor took.
- A side is an *energy side* when the neighbour is not a cable and
  `EnergyUtil.hasEnergy(...)` (i.e. exposes FE). It is recomputed on placement
  and on neighbour changes. **Different tiers never connect.**
- The six blockstate booleans are visual: they also include adjacent cables of
  the same tier.
- Block: `noCollission()`, hardness 2.0, resistance 20.0, metal sounds; shape is
  a 3.5-pixel core with 3-pixel arms.

### Blocks/items (content facts)

| id | class | hardness / resistance | states | notes |
|---|---|---|---|---|
| `powah:furnator_starter` | `AbstractGeneratorBlock` | 2.0 / 20.0, metal | `facing` (horizontal) | item stack size 1; block item is an `EnergyBlockItem` |
| `powah:energy_cell_starter` | `AbstractEnergyBlock` | 2.0 / 20.0, metal | none | item stack size 1, `EnergyCellItem` |
| `powah:energy_cable_starter` | `AbstractEnergyBlock` | 2.0 / 20.0, metal, no collision | `north,east,south,west,up,down` | item stacks to 1? (default) |

All three are in `#minecraft:mineable/pickaxe` + `#minecraft:needs_iron_tool`
(upstream's tool tags) and the loot is self-drop with `survives_explosion`.
Crafting recipes use `powah:capacitor_basic_tiny`, `powah:dielectric_casing`,
`powah:dielectric_paste` (later slices).

## Slice 1 boundary

Implemented: the starter **Furnator** (fuel → FE), the starter **Energy Cell**
(storage) and the starter **Energy Cable** (FE between adjacent block entities).

Gaps (recorded, not claimed): the other tiers and the creative cell; the
Magmator/Thermo/Solar/Reactors; Ender Cells/Gates; Energizing Orb/Rods;
Discharger/Hopper/Player Transmitter; item charging (`item_energy_buffer`) and
energy-carrying block items; side-config GUI and redstone modes (defaults only);
the furnator GUI; particles/sound; the cable **network** (the port moves energy
one cable hop, so a chain of cables does not relay); cable connection states
beyond the content tag; comparator output; worldgen (uraninite, dry ice).
