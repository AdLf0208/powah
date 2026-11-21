#!/usr/bin/env python3
"""Carry slice 6's content out of Powah 6.2.10's ATM10 jar into this package.

Slice 6 completes the two ladders slice 1 opened: the six remaining **furnator**
tiers and the six remaining **energy cable** tiers. Their assets, models and
loot are in the raw carry; their cable recipes were skipped by the carry (it
skipped the cable block whole, as slice 1's `CONTENT.md` records), so those are
read from the jar itself.

    python3 tools/carry_slice6.py           # write the files
    python3 tools/carry_slice6.py --check   # verify the tree already matches

What is patched, and why:

* `data/powah/horizon/block/*.json` — the carry's guesses are not this port's
  shapes: a furnator is the starter's `minecraft:furnace` definition (horizontal
  `facing`, 2.0/20.0 metal, `occludes: false`, stack 1) and a cable is the
  starter's `minecraft:chorus_plant` relay (`connects_to`, no collision). Both
  are copied from the package's own starter definitions, so a tier differs from
  the starter only in its id.
* `assets/powah/blockstates/furnator_*.json` — the carry's file still names
  `lollipop:block/empty` for `facing=up/down` and has no `lit` variant; the
  package's own starter blockstate (four facings x `lit`) is the one 26.3 loads.
* `data/powah/recipe/crafting/*.json` — the pin's loader needs the string
  ingredient form, and the cable recipes come from the jar.
* tags and the creative tab — the twelve ids join the families the starter
  already belongs to.

No asset or model is invented: every copied file is byte-for-byte the carry's.
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
CARRY = PORT.parent / "_shared" / "carry-work" / "powah" / "carry"
JAR = PORT.parent / "_shared" / "carry-work" / "powah" / "Powah-6.2.10.jar"

TIERS = ["basic", "hardened", "blazing", "niotic", "spirited", "nitro"]
FURNATORS = [f"furnator_{tier}" for tier in TIERS]
CABLES = [f"energy_cable_{tier}" for tier in TIERS]
BLOCKS = FURNATORS + CABLES
# Both of the jar's cable recipes per tier: the upgrade from the previous tier
# (`cable_<tier>`, 6) and the from-scratch alternate (`cable_<tier>_2`, 12).
CABLE_RECIPES = [f"cable_{tier}" for tier in TIERS] + [
    f"cable_{tier}_2" for tier in TIERS
]
FURNATOR_RECIPES = [f"furnator_{tier}" for tier in TIERS]

TAB = "data/powah/horizon/creative_tab/machines.json"
MINEABLE = "data/minecraft/tags/block/mineable/pickaxe.json"
NEEDS_IRON = "data/minecraft/tags/block/needs_iron_tool.json"
CONNECTS = "data/powah/tags/block/energy_cable_connects.json"
TAGS_TO_CARRY = [
    "data/powah/tags/block/furnators.json",
    "data/powah/tags/item/furnators.json",
    "data/powah/tags/block/energy_cables.json",
    "data/powah/tags/item/energy_cables.json",
]


def ingredient(value) -> str:
    """One recipe ingredient in the pin's string form."""
    if isinstance(value, str):
        if value.startswith("#"):
            raise SystemExit(f"a tag ingredient this slice does not carry: {value}")
        return value
    if "item" in value:
        return value["item"]
    raise SystemExit(f"a tag ingredient this slice does not carry: {value}")


def shaped(raw: dict) -> dict:
    """The jar's recipe re-shaped for this pin's loader."""
    out = {"category": raw.get("category", "misc"), "type": raw["type"]}
    out["key"] = {symbol: ingredient(value) for symbol, value in raw["key"].items()}
    out["pattern"] = raw["pattern"]
    out["result"] = raw["result"]
    return out


def jar_recipe(recipe_id: str) -> dict:
    """A recipe the raw carry skipped, read from the jar itself."""
    with zipfile.ZipFile(JAR) as jar:
        raw = json.loads(jar.read(f"data/powah/recipe/crafting/{recipe_id}.json"))
    return shaped(raw)


def carry_recipe(recipe_id: str) -> dict:
    raw = json.loads((CARRY / "data/powah/recipe/crafting" / f"{recipe_id}.json").read_text())
    return shaped(raw)


def outputs() -> dict[str, bytes]:
    """Every file this slice writes, as `path -> bytes`."""
    files: dict[str, bytes] = {}

    def copy(relative: str) -> None:
        source = CARRY / relative
        if not source.is_file():
            raise SystemExit(f"missing in the carry: {relative}")
        files[relative] = source.read_bytes()

    def write(relative: str, value) -> None:
        files[relative] = (json.dumps(value, indent=2) + "\n").encode()

    # The package's own starter definitions, which every tier repeats.
    furnator_definition = json.loads(
        (PORT / "data/powah/horizon/block/furnator_starter.json").read_text()
    )
    cable_definition = json.loads(
        (PORT / "data/powah/horizon/block/energy_cable_starter.json").read_text()
    )
    furnator_state = (PORT / "assets/powah/blockstates/furnator_starter.json").read_text()

    for block in FURNATORS:
        copy(f"assets/powah/models/block/{block}.json")
        copy(f"assets/powah/models/item/{block}.json")
        copy(f"assets/powah/items/{block}.json")
        copy(f"assets/powah/textures/block/{block.removeprefix('furnator_')}_ov.png")
        copy(f"data/powah/loot_table/blocks/{block}.json")
        write(f"data/powah/horizon/block/{block}.json", furnator_definition)
        tier = block.removeprefix("furnator_")
        files[f"assets/powah/blockstates/{block}.json"] = furnator_state.replace(
            "furnator_starter", block
        ).encode()
    for block in CABLES:
        copy(f"assets/powah/blockstates/{block}.json")
        copy(f"assets/powah/models/block/{block}.json")
        copy(f"assets/powah/models/block/{block}_multipart.json")
        copy(f"assets/powah/models/item/{block}.json")
        copy(f"assets/powah/items/{block}.json")
        copy(f"assets/powah/textures/block/{block}.png")
        copy(f"data/powah/loot_table/blocks/{block}.json")
        write(f"data/powah/horizon/block/{block}.json", cable_definition)

    for recipe_id in FURNATOR_RECIPES:
        write(f"data/powah/recipe/crafting/{recipe_id}.json", carry_recipe(recipe_id))
        copy(f"data/powah/advancement/recipes/misc/crafting/{recipe_id}.json")
    for recipe_id in CABLE_RECIPES:
        write(f"data/powah/recipe/crafting/{recipe_id}.json", jar_recipe(recipe_id))
        copy(f"data/powah/advancement/recipes/misc/crafting/{recipe_id}.json")

    for relative in TAGS_TO_CARRY:
        copy(relative)

    # The creative tab keeps the starter's row first, then the tiers this slice
    # adds, beside each family's starter.
    tab = json.loads((PORT / TAB).read_text())
    for index, item in enumerate(FURNATORS):
        entry = f"powah:{item}"
        if entry not in tab["items"]:
            tab["items"].insert(tab["items"].index("powah:furnator_starter") + 1 + index, entry)
    for index, item in enumerate(CABLES):
        entry = f"powah:{item}"
        if entry not in tab["items"]:
            tab["items"].insert(tab["items"].index("powah:energy_cable_starter") + 1 + index, entry)
    write(TAB, tab)

    # The pickaxe and iron-tool tags the starter machines are in.
    for relative in (MINEABLE, NEEDS_IRON):
        tag = json.loads((PORT / relative).read_text())
        for block in BLOCKS:
            entry = f"powah:{block}"
            if entry not in tag["values"]:
                tag["values"].append(entry)
        write(relative, tag)

    # A cable's visual arms join FE blocks; the new tiers join the starter's list.
    connects = json.loads((PORT / CONNECTS).read_text())
    for block in BLOCKS:
        entry = f"powah:{block}"
        if entry not in connects["values"]:
            connects["values"].append(entry)
    write(CONNECTS, connects)
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
            print(f"slice 6 content is stale ({len(stale)} file(s)):", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            return 1
        print(f"slice 6 content is reproducible: {len(files)} files match")
        return 0
    print(f"wrote {len(files)} files for slice 6")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
