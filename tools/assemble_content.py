#!/usr/bin/env python3
"""Assemble this port's `.hmod` root from the raw carry of the ATM10 jar.

The carry (`ports/_shared/carry-work/powah/carry`, produced by the pin's
`horizon-port-pack check`) holds every asset/data file the jar ships, upgraded
to 26.3. This script copies **only** what slice 1 needs and patches the handful
of files the carry gets wrong, so `content/` is reproducible and no asset is
invented:

| patched | why |
|---|---|
| `data/powah/horizon/block/*.json` | the carry's template guess (`carved_pumpkin`, `stone`) and its left-out cable; written from each block's real class and states |
| `assets/powah/blockstates/furnator_starter.json` | dropped the `lollipop:block/empty` model of another mod; covers the furnace template's `facing` x `lit` |
| `assets/powah/models/block/energy_cell.json` | its parent `block` (1.21.1's `models/block.json`) is not in 26.3; `block/block` is |
| `data/minecraft/tags/block/{mineable/pickaxe,needs_iron_tool}.json` | the carry lists all 92 blocks; only the three exist here |
| `data/powah/tags/block/energy_cable_connects.json` | new: the blocks the cable joins (upstream joins any FE block; content can only name blocks) |
| `data/powah/horizon/creative_tab/machines.json` | upstream's tab is code; one tab with the three items |
| `data/powah/recipe/crafting/cable_starter.json` | the carry skipped it (its block was left out); copied from the jar in the carry's upgraded shape |
| `data/powah/advancement/recipes/misc/crafting/cable_starter.json` | the matching recipe advancement, from the jar |
| `horizon-mod.toml` | this port's manifest |

Everything else is copied byte-for-byte from the carry.
"""
from pathlib import Path
import json
import shutil

ROOT = Path(__file__).resolve().parents[1]          # ports/powah
CAMPAIGN = ROOT.parents[1]                          # campaign root
CARRY = CAMPAIGN / "ports/_shared/carry-work/powah/carry"
OUT = ROOT / "content"

BLOCKS = ("furnator_starter", "energy_cell_starter", "energy_cable_starter")

# Copied as they are. The parent models (`furnator`, `energy_cell`, `cable`,
# `cable_multipart`) are the carried Blockbench models the three variants
# inherit; the textures are exactly the ones those models name.
CARRY_FILES = [
    "assets/powah/blockstates/energy_cell_starter.json",
    "assets/powah/blockstates/energy_cable_starter.json",
    "assets/powah/items/furnator_starter.json",
    "assets/powah/items/energy_cell_starter.json",
    "assets/powah/items/energy_cable_starter.json",
    "assets/powah/lang/en_us.json",
    "assets/powah/models/block/cable.json",
    "assets/powah/models/block/cable_multipart.json",
    "assets/powah/models/block/energy_cable_starter.json",
    "assets/powah/models/block/energy_cable_starter_multipart.json",
    "assets/powah/models/block/energy_cell.json",
    "assets/powah/models/block/energy_cell_starter.json",
    "assets/powah/models/block/furnator.json",
    "assets/powah/models/block/furnator_starter.json",
    "assets/powah/models/item/energy_cable_starter.json",
    "assets/powah/models/item/energy_cell_starter.json",
    "assets/powah/models/item/furnator_starter.json",
    "assets/powah/textures/block/energy_cable_starter.png",
    "assets/powah/textures/block/energy_cell_starter.png",
    "assets/powah/textures/block/furnator_face.png",
    "assets/powah/textures/block/furnator_side.png",
    "assets/powah/textures/block/furnator_top.png",
    "assets/powah/textures/block/furnator_unlit.png",
    "assets/powah/textures/block/starter_ov.png",
    "data/powah/advancement/recipes/misc/crafting/energy_cell_starter.json",
    "data/powah/advancement/recipes/misc/crafting/furnator_starter.json",
    "data/powah/loot_table/blocks/energy_cable_starter.json",
    "data/powah/loot_table/blocks/energy_cell_starter.json",
    "data/powah/loot_table/blocks/furnator_starter.json",
    "data/powah/recipe/crafting/energy_cell_starter.json",
    "data/powah/recipe/crafting/furnator_starter.json",
]

# Upstream's furnator faces the player (`isPlacerFacing`, horizontal facing). The
# furnace template's `lit` is never set in this slice (upstream's `burning` is a
# sync flag, not a block state), so both values bind the same model.
FURNATOR_BLOCKSTATE = {
    "variants": {
        f"facing={facing},lit={lit}": {
            "model": "powah:block/furnator_starter",
            **({"y": yaw} if yaw else {}),
        }
        for facing, yaw in (("north", 0), ("south", 180), ("west", 270), ("east", 90))
        for lit in ("false", "true")
    }
}

BLOCK_DECLS = {
    # `Properties.metalNoSolid(2.0, 20.0)`: metal sounds, requires a tool, and no
    # occlusion. The furnator keeps the furnace's horizontal `facing`.
    "furnator_starter": {
        "template": "minecraft:furnace",
        "properties": {
            "hardness": 2.0,
            "explosion_resistance": 20.0,
            "sounds": "minecraft:iron_block",
            "requires_tool": True,
            "occludes": False,
        },
        "item": True,
        "components": {"minecraft:max_stack_size": 1},
    },
    # No states of its own; the cell model is its own full shape.
    "energy_cell_starter": {
        "template": "minecraft:iron_block",
        "properties": {
            "hardness": 2.0,
            "explosion_resistance": 20.0,
            "sounds": "minecraft:iron_block",
            "requires_tool": True,
            "occludes": False,
        },
        "item": True,
        "components": {"minecraft:max_stack_size": 1},
    },
    # `PipeBlock`: six sides joined to the tag's blocks and itself, with the core
    # and arms `thickness` builds; `noCollission()`.
    "energy_cable_starter": {
        "template": "minecraft:chorus_plant",
        "properties": {
            "hardness": 2.0,
            "explosion_resistance": 20.0,
            "sounds": "minecraft:iron_block",
            "requires_tool": True,
            "occludes": False,
        },
        "connects_to": "#powah:energy_cable_connects",
        "thickness": 4.0,
        "collision_shapes": {"default": []},
        "item": True,
    },
}

MANIFEST = """\
# Slice 1 of the Powah port: the starter Furnator, Energy Cell and Energy Cable.
# Assets and data are carried from the ATM10 jar (Powah-6.2.10.jar), with the
# patches documented in README.md. Licence: LGPL-3.0-only (Technici4n/Powah);
# local-only until the maintainer approves publishing.
id = "powah"
name = "Powah"
version = "6.2.10"
description = "Starter Furnator, Energy Cell and Energy Cable (slice 1)"
authors = ["owmii", "Technici4n"]
api = "0.2"
side = "both"
capabilities = ["registries"]
"""

CABLE_RECIPE = {
    "type": "minecraft:crafting_shaped",
    "category": "misc",
    "key": {
        "d": "powah:dielectric_rod_horizontal",
        "i": "minecraft:iron_nugget",
        "t": "powah:capacitor_basic_tiny",
    },
    "pattern": ["ddd", "iti", "ddd"],
    "result": {"count": 12, "id": "powah:energy_cable_starter"},
}

CABLE_ADVANCEMENT = {
    "parent": "minecraft:recipes/root",
    "criteria": {
        "has_capacitor_basic_tiny": {
            "conditions": {"items": [{"items": "powah:capacitor_basic_tiny"}]},
            "trigger": "minecraft:inventory_changed",
        },
        "has_dielectric_rod_horizontal": {
            "conditions": {"items": [{"items": "powah:dielectric_rod_horizontal"}]},
            "trigger": "minecraft:inventory_changed",
        },
        "has_iron_nugget": {
            "conditions": {"items": [{"items": "minecraft:iron_nugget"}]},
            "trigger": "minecraft:inventory_changed",
        },
        "has_the_recipe": {
            "conditions": {"recipe": "powah:crafting/cable_starter"},
            "trigger": "minecraft:recipe_unlocked",
        },
    },
    "requirements": [
        [
            "has_the_recipe",
            "has_dielectric_rod_horizontal",
            "has_iron_nugget",
            "has_capacitor_basic_tiny",
        ]
    ],
    "rewards": {"recipes": ["powah:crafting/cable_starter"]},
}


def write(relative: str, value) -> None:
    path = OUT / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    text = value if isinstance(value, str) else json.dumps(value, indent=2) + "\n"
    path.write_text(text)


def main() -> None:
    if not (CARRY / "horizon-mod.toml").exists():
        raise SystemExit(f"missing carry: {CARRY}")
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir()
    for relative in CARRY_FILES:
        source = CARRY / relative
        if not source.exists():
            raise SystemExit(f"missing carried file: {relative}")
        target = OUT / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)

    write("horizon-mod.toml", MANIFEST)
    for name, decl in BLOCK_DECLS.items():
        write(f"data/powah/horizon/block/{name}.json", decl)
    write("assets/powah/blockstates/furnator_starter.json", FURNATOR_BLOCKSTATE)
    write(
        "data/powah/horizon/creative_tab/machines.json",
        {
            "title": "itemGroup.powah.tab",
            "icon": "powah:energy_cell_starter",
            "items": [f"powah:{name}" for name in BLOCKS],
        },
    )
    write(
        "data/minecraft/tags/block/mineable/pickaxe.json",
        {"values": [f"powah:{name}" for name in BLOCKS]},
    )
    write(
        "data/minecraft/tags/block/needs_iron_tool.json",
        {"values": [f"powah:{name}" for name in BLOCKS]},
    )
    # Upstream joins any block with an FE capability; a content tag can only name
    # blocks, so the slice's three are the ones the cable draws arms to.
    write(
        "data/powah/tags/block/energy_cable_connects.json",
        {"values": [f"powah:{name}" for name in BLOCKS]},
    )
    write("data/powah/recipe/crafting/cable_starter.json", CABLE_RECIPE)
    write(
        "data/powah/advancement/recipes/misc/crafting/cable_starter.json",
        CABLE_ADVANCEMENT,
    )

    # 1.21.1 shipped `assets/minecraft/models/block.json`; 26.3 has only
    # `block/block`, so the carried cell model's bare `block` parent is missing.
    cell_model = (OUT / "assets/powah/models/block/energy_cell.json").read_text()
    if '"parent": "block"' not in cell_model:
        raise SystemExit("the carried energy cell model no longer names 'block'")
    write(
        "assets/powah/models/block/energy_cell.json",
        cell_model.replace('"parent": "block"', '"parent": "block/block"'),
    )

    write(
        "README.md",
        "# Powah — content (slice 1)\n\n"
        "Built by `tools/assemble_content.py`; the patch table and the build\n"
        "command are in [`../CONTENT.md`](../CONTENT.md). Do not edit these files\n"
        "by hand: rerun the script.\n",
    )

    files = sum(1 for path in OUT.rglob("*") if path.is_file())
    print(f"assembled {OUT} ({files} files)")


if __name__ == "__main__":
    main()
