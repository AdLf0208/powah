#!/usr/bin/env python3
"""Carry slice 5's content out of Powah 6.2.10's ATM10 jar into this package.

Slice 1–4 carried their files by hand from the raw carry; this slice adds 20 new
ids (the Energizing Orb, seven rods, the five materials and six capacitors) plus
one material block, so the carry is done by a script that can be re-run and
checked instead of by 160 hand copies.

    python3 tools/carry_slice5.py           # write the files
    python3 tools/carry_slice5.py --check   # verify the tree already matches

The source is the carry of `Powah-6.2.10.jar` produced by `horizon-port-pack`
(see `CONTENT.md` for provenance). Only the paths this slice names are read;
assets, models and textures are copied byte-for-byte, and the four kinds of file
that need a change are written from the shapes the package already uses:

* `data/powah/horizon/block/*.json` — the orb and the rods need the six-value
  `facing` state upstream's `Facing.ALL` gives them (the carry guessed a carved
  pumpkin, which has only four), and every block is upstream's 2.0/20.0 metal;
* `data/powah/horizon/item/*.json` — plain 64-stack items, as slice 2's are;
* `data/powah/recipe/crafting/*.json` — the string ingredient form the pin's
  loader needs, and the NeoForge `c:` tags replaced by the vanilla member they
  resolve to (26.3 has no `c:` tags; see `notes.md` slice 5);
* `data/powah/advancement/...` — the same tag replacement, so each recipe still
  unlocks when its ingredients are in the inventory.
"""
from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
SOURCE = PORT.parent / "_shared" / "carry-work" / "powah" / "carry"

# --- what this slice adds ---------------------------------------------------

BLOCKS = [
    "energizing_orb",
    "energizing_rod_starter",
    "energizing_rod_basic",
    "energizing_rod_hardened",
    "energizing_rod_blazing",
    "energizing_rod_niotic",
    "energizing_rod_spirited",
    "energizing_rod_nitro",
    "blazing_crystal_block",
]
ITEMS = [
    "steel_energized",
    "crystal_blazing",
    "crystal_niotic",
    "crystal_spirited",
    "crystal_nitro",
    "capacitor_basic_large",
    "capacitor_hardened",
    "capacitor_blazing",
    "capacitor_niotic",
    "capacitor_spirited",
    "capacitor_nitro",
]
# The one recipe file per id, copied from the jar and re-shaped.
RECIPES = [
    "energizing_orb",
    "energizing_rod_starter",
    "energizing_rod_basic",
    "energizing_rod_hardened",
    "energizing_rod_blazing",
    "energizing_rod_niotic",
    "energizing_rod_spirited",
    "energizing_rod_nitro",
    "capacitor_basic_large",
    "capacitor_hardened",
    "capacitor_blazing",
    "capacitor_niotic",
    "capacitor_spirited",
    "capacitor_nitro",
    "energy_cell_hardened",
    "energy_cell_blazing",
    "energy_cell_niotic",
    "energy_cell_spirited",
    "energy_cell_nitro",
    "blazing_crystal_block",
    "blazing_crystal",
]
# NeoForge's common tags have no 26.3 equivalent: each is carried as the vanilla
# id(s) the tag resolves to in a vanilla world.
TAGS = {
    "c:glass_blocks": "minecraft:glass",
    "c:storage_blocks/quartz": "minecraft:quartz_block",
}
# The block textures: the orb's three, the rod's shared one plus its seven gems,
# and the material block's.
BLOCK_TEXTURES = [
    "energizing_orb.png",
    "energizing_orb_base.png",
    "energizing_orb_glass.png",
    "energizing_rod.png",
    "energizing_rod_starter_gem.png",
    "energizing_rod_basic_gem.png",
    "energizing_rod_hardened_gem.png",
    "energizing_rod_blazing_gem.png",
    "energizing_rod_niotic_gem.png",
    "energizing_rod_spirited_gem.png",
    "energizing_rod_nitro_gem.png",
    "blazing_crystal_block.png",
]
ROD_TIERS = [
    "starter",
    "basic",
    "hardened",
    "blazing",
    "niotic",
    "spirited",
    "nitro",
]
# Upstream's `Properties.metal(2.0, 20.0)`, and `metalNoSolid` for a block the
# player can see through (the orb and the rods).
METAL = {
    "hardness": 2.0,
    "explosion_resistance": 20.0,
    "sounds": "minecraft:iron_block",
    "requires_tool": True,
}
FACING = {
    "type": "enum",
    "values": ["down", "up", "north", "south", "west", "east"],
}
TAB = "data/powah/horizon/creative_tab/machines.json"
TAB_ITEMS = [
    "powah:energizing_orb",
    *[f"powah:energizing_rod_{tier}" for tier in ROD_TIERS],
    "powah:steel_energized",
    "powah:crystal_blazing",
    "powah:crystal_niotic",
    "powah:crystal_spirited",
    "powah:crystal_nitro",
    "powah:blazing_crystal_block",
    "powah:capacitor_basic_large",
    "powah:capacitor_hardened",
    "powah:capacitor_blazing",
    "powah:capacitor_niotic",
    "powah:capacitor_spirited",
    "powah:capacitor_nitro",
]
TAGS_TO_CARRY = [
    "data/powah/tags/block/energizing_rods.json",
    "data/powah/tags/item/energizing_rods.json",
]
MINEABLE = "data/minecraft/tags/block/mineable/pickaxe.json"


def block_definition(block: str) -> dict:
    """The block's `horizon/block` file, from upstream's own block class."""
    if block == "blazing_crystal_block":
        # `new Block(Properties.metal(2.0f, 20.0f))`: a solid metal block, 64-stack.
        return {
            "template": "minecraft:stone",
            "properties": dict(METAL),
            "item": True,
        }
    # `EnergizingOrbBlock` / `EnergizingRodBlock`: `metalNoSolid` with the six-way
    # `facing` state (`Facing.ALL`, default DOWN), item stacks to 1.
    return {
        "template": "minecraft:iron_block",
        "properties": {**METAL, "occludes": False},
        "state_properties": {"facing": FACING},
        "default_state": {"facing": "down"},
        "item": True,
        "components": {"minecraft:max_stack_size": 1},
    }


def ingredient(value) -> str:
    """One recipe ingredient in the pin's string form."""
    if isinstance(value, str):
        if value.startswith("#"):
            tag = value[1:]
            if tag not in TAGS:
                raise SystemExit(f"no 26.3 member recorded for the tag {tag}")
            return TAGS[tag]
        return value
    if "item" in value:
        return value["item"]
    tag = value["tag"]
    if tag not in TAGS:
        raise SystemExit(f"no 26.3 member recorded for the tag {tag}")
    return TAGS[tag]


def recipe(recipe_id: str) -> dict:
    """The jar's recipe re-shaped for this pin's loader."""
    raw = json.loads((SOURCE / "data/powah/recipe/crafting" / f"{recipe_id}.json").read_text())
    out = {"category": raw.get("category", "misc"), "type": raw["type"]}
    if "key" in raw:
        out["key"] = {symbol: ingredient(value) for symbol, value in raw["key"].items()}
        out["pattern"] = raw["pattern"]
    else:
        out["ingredients"] = [ingredient(value) for value in raw["ingredients"]]
    out["result"] = raw["result"]
    return out


def advancement(recipe_id: str) -> dict:
    """The jar's recipe-unlock advancement, with the same tag replacement."""
    path = SOURCE / "data/powah/advancement/recipes/misc/crafting" / f"{recipe_id}.json"
    data = json.loads(path.read_text())
    text = json.dumps(data)
    for tag, member in TAGS.items():
        text = text.replace(f'"#{tag}"', f'"{member}"')
    return json.loads(text)


def creative_tab() -> dict:
    tab = json.loads((PORT / TAB).read_text())
    for item in TAB_ITEMS:
        if item not in tab["items"]:
            tab["items"].append(item)
    return tab


def outputs() -> dict[str, bytes]:
    """Every file this slice writes, as `path -> bytes`."""
    files: dict[str, bytes] = {}

    def copy(relative: str) -> None:
        source = SOURCE / relative
        if not source.is_file():
            raise SystemExit(f"missing in the carry: {relative}")
        files[relative] = source.read_bytes()

    def write(relative: str, value) -> None:
        files[relative] = (json.dumps(value, indent=2) + "\n").encode()

    for block in BLOCKS:
        copy(f"assets/powah/blockstates/{block}.json")
        copy(f"assets/powah/models/block/{block}.json")
        copy(f"assets/powah/models/item/{block}.json")
        copy(f"assets/powah/items/{block}.json")
        copy(f"data/powah/loot_table/blocks/{block}.json")
        write(f"data/powah/horizon/block/{block}.json", block_definition(block))
    # The rods' block models all parent this one.
    copy("assets/powah/models/block/energizing_rod.json")
    for item in ITEMS:
        copy(f"assets/powah/models/item/{item}.json")
        copy(f"assets/powah/items/{item}.json")
        write(f"data/powah/horizon/item/{item}.json", {"template": "minecraft:stick"})
    for texture in BLOCK_TEXTURES:
        copy(f"assets/powah/textures/block/{texture}")
    for item in ITEMS:
        copy(f"assets/powah/textures/item/{item}.png")
        # Three of the five materials are animated, and their `.mcmeta` says so.
        animated = SOURCE / f"assets/powah/textures/item/{item}.png.mcmeta"
        if animated.is_file():
            copy(f"assets/powah/textures/item/{item}.png.mcmeta")
    for recipe_id in RECIPES:
        write(f"data/powah/recipe/crafting/{recipe_id}.json", recipe(recipe_id))
        write(
            f"data/powah/advancement/recipes/misc/crafting/{recipe_id}.json",
            advancement(recipe_id),
        )
    for relative in TAGS_TO_CARRY:
        copy(relative)
    write(TAB, creative_tab())
    # The nine new blocks join the jar's pickaxe tag. `needs_iron_tool` is left as
    # slices 1–4 have it (upstream's own file lists only the ores; see notes.md).
    mineable = json.loads((PORT / MINEABLE).read_text())
    for block in BLOCKS:
        entry = f"powah:{block}"
        if entry not in mineable["values"]:
            mineable["values"].append(entry)
    write(MINEABLE, mineable)
    return files


def main() -> int:
    check = "--check" in sys.argv[1:]
    files = outputs()
    stale = []
    for relative, content in sorted(files.items()):
        path = PORT / relative
        if check:
            if not path.is_file():
                stale.append(f"missing {relative}")
            elif path.read_bytes() != content:
                stale.append(f"differs {relative}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
    if check:
        if stale:
            print(f"slice 5 content is stale ({len(stale)} file(s)):", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            return 1
        print(f"slice 5 content is reproducible: {len(files)} files match")
        return 0
    print(f"wrote {len(files)} files for slice 5")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
