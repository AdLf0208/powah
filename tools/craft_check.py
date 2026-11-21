#!/usr/bin/env python3
"""Craft Powah's slice-2 recipes in a real crafting table, and the machine recipes.

Called by `tools/e2e.sh` with the open game's control socket and a crafting
table's position. The harness numbers a crafting table's grid 100-108
row-major and its result slot 200 (the `additional_lights` pattern), so each
case below places the recipe's declared ingredients in the slots the pattern
names, in survival so the grid is really consumed, and asserts:

1. the result slot holds exactly the recipe's output id and count;
2. taking it puts that count in the inventory;
3. every ingredient's inventory count fell by exactly what the recipe used
   (`dielectric_paste` also asserts the lava bucket leaves a bucket in the
   grid, i.e. the crafting remainder);
4. an off-pattern grid (a stick where the casing belongs) leaves the result
   empty.

The cases are the eight material recipes slice 2 adds, the three machine recipes
slice 1 carried but could not craft, slice 4's basic energy cell, and slice 5's
twenty-one: the energizing orb, the seven rods, the large capacitor and its five
tiers, the five cell recipes that needed the orb's materials, and the blazing
crystal block both ways. A wrong id or count exits 1 with the observed slot and
two off-pattern grids (a stick where the casing goes, on the furnator's and the
orb's shapes) must leave the result empty. `SELFTEST=1` expects the first output
one item too high, so the run MUST fail — that is what proves the assertions can
fail.
"""
import json
import os
import subprocess
import sys

# A crafting table's grid, row-major (the harness's numbering).
TOP_LEFT = 100
RESULT = 200
GRID = range(100, 109)

# The two machine recipes' patterns, as slot lists (see the module docstring).
FURNATOR = {"powah:dielectric_paste": [100, 101, 102, 106, 108],
            "powah:capacitor_basic_tiny": [103, 105],
            "powah:dielectric_casing": [104],
            "minecraft:furnace": [107]}
CELL = {"minecraft:iron_ingot": [100, 102, 106, 108],
        "powah:capacitor_basic_tiny": [101, 103, 105, 107],
        "powah:dielectric_casing": [104]}
CABLE = {"powah:dielectric_rod_horizontal": [100, 101, 102, 106, 107, 108],
         "minecraft:iron_nugget": [103, 105],
         "powah:capacitor_basic_tiny": [104]}
ROD = {"powah:dielectric_paste": [100, 102, 103, 105, 106, 108],
       "minecraft:iron_bars": [101, 104, 107]}
ROD_H = {"powah:dielectric_paste": [100, 101, 102, 106, 107, 108],
         "minecraft:iron_bars": [103, 104, 105]}
CAPACITOR = {"minecraft:iron_ingot": [101, 103, 105, 107],
             "powah:dielectric_paste": [102, 106],
             "minecraft:redstone_block": [104]}
# `ici/cdc/ici`: 4 iron, 4 basic capacitors, one casing (slice 4's basic cell).
CELL_BASIC = {"minecraft:iron_ingot": [100, 102, 106, 108],
              "powah:capacitor_basic": [101, 103, 105, 107],
              "powah:dielectric_casing": [104]}

# --- slice 5: the energizing orb, the rod ladder and the materials ------------

# `ggg/gcg/rrr`: 5 glass blocks, a casing, 3 horizontal dielectric rods.
ENERGIZING_ORB = {"minecraft:glass": [100, 101, 102, 103, 105],
                  "powah:dielectric_casing": [104],
                  "powah:dielectric_rod_horizontal": [106, 107, 108]}


def rod(capacitor, below, quartz="minecraft:quartz_block"):
    """`" q "/"bcb"/" h "`: a quartz block, two capacitors, a casing and the
    rod the tier is built on. The starter is the jar's exception: its `q` is a
    piece of quartz, not a quartz block."""
    return {quartz: [101],
            capacitor: [103, 105],
            "powah:dielectric_casing": [104],
            below: [107]}


# `" q "/"hch"/" e "`: two capacitors of the tier in the middle row (the casing
# sits between them), a quartz block and the previous rod in the bottom row.
ROD_HARDENED = {"minecraft:quartz_block": [101],
                "powah:capacitor_hardened": [103, 105],
                "powah:dielectric_casing": [104],
                "powah:energizing_rod_basic": [107]}


def capacitor(material):
    """`pbp/bcb/pbp`: four dielectric paste, four of the material and a large
    capacitor in the centre (the jar's `c` is `capacitor_basic_large`)."""
    return {"powah:dielectric_paste": [100, 102, 106, 108],
            material: [101, 103, 105, 107],
            "powah:capacitor_basic_large": [104]}


def cell(material, capacitor_id, below):
    """`m?m/cdc/m?m`: four of the tier's material, its capacitor in the middle
    column, the previous tier's cell beside the casing."""
    return {material: [100, 102, 106, 108],
            capacitor_id: [101, 107],
            below: [103, 105],
            "powah:dielectric_casing": [104]}


CELL_HARDENED = cell("powah:steel_energized", "powah:capacitor_hardened",
                     "powah:energy_cell_basic")


def shaped(label, pattern, out, out_count):
    """One shaped case: the pattern's slots, one item each, and the output."""
    return {"label": label, "place": pattern, "want": (out, out_count),
            "remainder": None}


def shapeless(label, cells, out, out_count, remainder=None):
    """One shapeless case: `cells` is [(id, slot)] — one item per grid cell.

    The grid holds one stack per cell and this pin matches a shapeless recipe's
    ingredients against the cells, not against the item count (measured: three
    coals in one cell never match three coal ingredients; three cells of one
    coal do), so every ingredient gets a cell of its own.
    """
    place = {}
    for item, slot in cells:
        place.setdefault(item, []).append(slot)
    return {"label": label, "place": place, "want": (out, out_count),
            "remainder": remainder}


CASES = [
    # --- the eight material recipes this slice adds -------------------------
    shapeless("3 coal + 2 clay + lava bucket -> 24 dielectric paste",
              [("minecraft:coal", 100), ("minecraft:coal", 101),
               ("minecraft:coal", 102), ("minecraft:clay_ball", 103),
               ("minecraft:clay_ball", 104), ("minecraft:lava_bucket", 105)],
              "powah:dielectric_paste", 24, remainder=("minecraft:bucket", 1)),
    shapeless("2 coal + clay + blaze powder -> 16 dielectric paste",
              [("minecraft:coal", 100), ("minecraft:coal", 101),
               ("minecraft:clay_ball", 102), ("minecraft:blaze_powder", 103)],
              "powah:dielectric_paste", 16),
    shaped("paste + iron bars -> 8 dielectric rods", ROD, "powah:dielectric_rod", 8),
    shaped("paste + iron bars -> 8 horizontal dielectric rods", ROD_H,
           "powah:dielectric_rod_horizontal", 8),
    shapeless("one horizontal rod -> one vertical rod",
              [("powah:dielectric_rod_horizontal", 100)],
              "powah:dielectric_rod", 1),
    shapeless("one vertical rod -> one horizontal rod",
              [("powah:dielectric_rod", 100)],
              "powah:dielectric_rod_horizontal", 1),
    shaped("iron + paste + redstone block -> 4 basic capacitors", CAPACITOR,
           "powah:capacitor_basic", 4),
    shapeless("one basic capacitor -> two tiny capacitors",
              [("powah:capacitor_basic", 100)],
              "powah:capacitor_basic_tiny", 2),
    # --- the three machine recipes slice 1 carried but could not craft ------
    shaped("3 paste + 2 tiny capacitors + casing + furnace -> a furnator",
           FURNATOR, "powah:furnator_starter", 1),
    shaped("4 iron + 4 tiny capacitors + casing -> an energy cell", CELL,
           "powah:energy_cell_starter", 1),
    shaped("6 horizontal rods + 2 nuggets + tiny capacitor -> 12 cables", CABLE,
           "powah:energy_cable_starter", 12),
    # --- slice 4: the basic cell, the one tier whose recipe this slice carries --
    shaped("4 iron + 4 basic capacitors + casing -> a basic energy cell",
           CELL_BASIC, "powah:energy_cell_basic", 1),
    # --- slice 5: the orb, the rod ladder and the materials it makes -----------
    shaped("5 glass + casing + 3 horizontal rods -> an energizing orb",
           ENERGIZING_ORB, "powah:energizing_orb", 1),
    shaped("quartz + 2 tiny capacitors + casing + rod -> a starter rod",
           rod("powah:capacitor_basic_tiny", "powah:dielectric_rod",
               quartz="minecraft:quartz"),
           "powah:energizing_rod_starter", 1),
    shaped("quartz + 2 capacitors + casing + starter rod -> a basic rod",
           rod("powah:capacitor_basic", "powah:energizing_rod_starter"),
           "powah:energizing_rod_basic", 1),
    shaped("quartz + 2 hardened capacitors + casing + basic rod -> a hardened rod",
           ROD_HARDENED, "powah:energizing_rod_hardened", 1),
    shaped("quartz + 2 blazing capacitors + casing + hardened rod -> a blazing rod",
           rod("powah:capacitor_blazing", "powah:energizing_rod_hardened"),
           "powah:energizing_rod_blazing", 1),
    shaped("quartz + 2 niotic capacitors + casing + blazing rod -> a niotic rod",
           rod("powah:capacitor_niotic", "powah:energizing_rod_blazing"),
           "powah:energizing_rod_niotic", 1),
    shaped("quartz + 2 spirited capacitors + casing + niotic rod -> a spirited rod",
           rod("powah:capacitor_spirited", "powah:energizing_rod_niotic"),
           "powah:energizing_rod_spirited", 1),
    shaped("quartz + 2 nitro capacitors + casing + spirited rod -> a nitro rod",
           rod("powah:capacitor_nitro", "powah:energizing_rod_spirited"),
           "powah:energizing_rod_nitro", 1),
    shapeless("two basic capacitors -> one large capacitor",
              [("powah:capacitor_basic", 100), ("powah:capacitor_basic", 101)],
              "powah:capacitor_basic_large", 1),
    shaped("paste + energized steel + large capacitor -> 2 hardened capacitors",
           capacitor("powah:steel_energized"), "powah:capacitor_hardened", 2),
    shaped("paste + blazing crystal + large capacitor -> 2 blazing capacitors",
           capacitor("powah:crystal_blazing"), "powah:capacitor_blazing", 2),
    shaped("paste + niotic crystal + large capacitor -> a niotic capacitor",
           capacitor("powah:crystal_niotic"), "powah:capacitor_niotic", 1),
    shaped("paste + spirited crystal + large capacitor -> a spirited capacitor",
           capacitor("powah:crystal_spirited"), "powah:capacitor_spirited", 1),
    shaped("paste + nitro crystal + large capacitor -> a nitro capacitor",
           capacitor("powah:crystal_nitro"), "powah:capacitor_nitro", 1),
    shaped("4 energized steel + 2 hardened capacitors + basic cell + casing -> a hardened cell",
           CELL_HARDENED, "powah:energy_cell_hardened", 1),
    shaped("4 blazing crystals + 2 blazing capacitors + hardened cell + casing -> a blazing cell",
           cell("powah:crystal_blazing", "powah:capacitor_blazing",
                "powah:energy_cell_hardened"), "powah:energy_cell_blazing", 1),
    shaped("4 niotic crystals + 2 niotic capacitors + blazing cell + casing -> a niotic cell",
           cell("powah:crystal_niotic", "powah:capacitor_niotic",
                "powah:energy_cell_blazing"), "powah:energy_cell_niotic", 1),
    shaped("4 spirited crystals + 2 spirited capacitors + niotic cell + casing -> a spirited cell",
           cell("powah:crystal_spirited", "powah:capacitor_spirited",
                "powah:energy_cell_niotic"), "powah:energy_cell_spirited", 1),
    shaped("4 nitro crystals + 2 nitro capacitors + spirited cell + casing -> a nitro cell",
           cell("powah:crystal_nitro", "powah:capacitor_nitro",
                "powah:energy_cell_spirited"), "powah:energy_cell_nitro", 1),
    shapeless("nine blazing crystals -> one blazing crystal block",
              [("powah:crystal_blazing", slot) for slot in range(100, 109)],
              "powah:blazing_crystal_block", 1),
    shapeless("one blazing crystal block -> nine blazing crystals",
              [("powah:blazing_crystal_block", 100)],
              "powah:crystal_blazing", 9),
]


def main():
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
    selftest = os.environ.get("SELFTEST") == "1"

    def run(*args):
        out = subprocess.run([ctl, "--socket", sock, "--timeout", "30", *args],
                             text=True, capture_output=True)
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=2):
        run("wait", str(ticks))

    def counts():
        return run("inventory").get("counts", {})

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def slot_of(number):
        return next((s for s in slots() if s["slot"] == number), None)

    def find(item):
        for slot in slots():
            if slot.get("item") == item and slot["slot"] < 100:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the inventory")

    def empty_slot():
        for slot in slots():
            if not slot.get("item") and slot["slot"] < 100:
                return slot["slot"]
        raise SystemExit("no empty inventory slot to take the result into")

    def setup(given):
        """Open a fresh crafting table with exactly `given` in the inventory.

        The interact is retried: it answers "no face of block ... is visible" or
        "out of reach" when the player drifted or fell, and every click after it
        would then land in the inventory instead of the grid — the failure this
        cost the slice-5 run (12 cases read "got None xNone" from a screen that
        was not the table).
        """
        try:
            run("close")
        except SystemExit:
            pass
        wait(3)
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        run("command", "gamemode survival @s")
        run("command", "clear @s")
        wait(2)
        for item, count in given:
            run("give", item, str(count))
        wait(3)
        # Stand two and a half blocks away and face the table: `interact` refuses
        # a face the player's eye cannot see, and standing right beside it puts
        # the eye above the block (measured: "no face of block ... is visible").
        for _ in range(4):
            run("command", f"tp @s {float(x) - 2.5} {y} {float(z) + 0.5}")
            run("look-at-block", x, y, z)
            wait(2)
            run("interact", x, y, z)
            wait(4)
            screen = run("screen")
            if any(slot["slot"] == RESULT for slot in screen.get("slots", [])):
                return
        raise SystemExit(f"could not open the crafting table at {x},{y},{z}: {screen}")

    def place(item, targets):
        """Place one ITEM into each target slot.

        A pickup puts as many into the grid as the stack holds: a right click with
        an empty cursor places nothing, and the cells, the rods and the orb stack
        to one, so one pickup per slot is what those recipes need.
        """
        remaining = list(targets)
        while remaining:
            source = find(item)
            stack = slot_of(source) or {}
            per_pickup = max(1, int(stack.get("count") or 1))
            run("click-slot", str(source))
            wait(1)
            for target in remaining[:per_pickup]:
                run("click-slot", str(target), "right")
                wait(1)
            del remaining[:per_pickup]
            run("click-slot", str(source))
            wait(2)

    failures = []

    def fail(message):
        failures.append(message)
        print(f"FAIL: {message}", file=sys.stderr)

    for index, case in enumerate(CASES):
        want_item, want_count = case["want"]
        # One item per grid slot, plus the count the pattern repeats a slot by.
        given = {}
        for item, targets in case["place"].items():
            given[item] = given.get(item, 0) + len(targets)
        given_list = sorted(given.items())

        setup(given_list)
        before = counts()
        for item, targets in case["place"].items():
            place(item, targets)

        result = slot_of(RESULT)
        got = result.get("item") if result else None
        got_count = result.get("count") if result else None
        expected = want_count + 1 if selftest and index == 0 else want_count
        print(f"{case['label']} -> {json.dumps(result)}")
        if got != want_item or got_count != expected:
            grid = [{"slot": s["slot"], "item": s.get("item")}
                    for s in slots() if 100 <= s["slot"] < 200]
            fail(f"{case['label']}: expected {expected} {want_item}, got {got} x{got_count}; "
                 f"grid {json.dumps(grid)}")
            if selftest and index == 0:
                # The selftest's whole point: the wrong count must be caught here.
                break
            run("close")
            continue
        if selftest and index == 0:
            fail(f"SELFTEST: the paste crafted {got_count}, the count this run "
                 f"expected — the assertion is not alive")
            break

        # Take the result, then assert the inventory moved by exactly the recipe.
        run("click-slot", str(RESULT))
        wait(2)
        run("click-slot", str(empty_slot()))
        wait(3)
        after = counts()
        made = after.get(want_item, 0) - before.get(want_item, 0)
        if made != want_count:
            fail(f"{case['label']}: the inventory gained {made} {want_item}, not {want_count}")
        for item, used in given.items():
            left = before.get(item, 0) - after.get(item, 0)
            if left != used:
                fail(f"{case['label']}: {item} fell by {left}, not the {used} it used")
        if case["remainder"]:
            item, count = case["remainder"]
            held = [s for s in slots() if s["slot"] in GRID and s.get("item") == item]
            if len(held) != count or held[0].get("count") != count:
                fail(f"{case['label']}: the grid does not hold {count} {item} as the "
                     f"crafting remainder (slots: {held})")
        run("close")
        wait(2)

    # The off-pattern controls: the furnator's shape with a stick where the
    # dielectric casing belongs must leave the result empty, and so must the orb's
    # shape with a stick where its casing belongs. The casing is in the inventory
    # but not in the grid, so only the stick is out of place.
    for label, pattern, stand_in in (
        ("furnator", FURNATOR, (104, "powah:dielectric_casing")),
        ("energizing orb", ENERGIZING_ORB, (104, "powah:dielectric_casing")),
    ):
        given = {}
        for item, targets in pattern.items():
            if targets == [stand_in[0]]:
                continue
            given[item] = len(targets)
        given["minecraft:stick"] = 1
        setup(sorted(given.items()))
        for item, targets in pattern.items():
            if targets == [stand_in[0]]:
                continue
            place(item, targets)
        place("minecraft:stick", [stand_in[0]])
        result = slot_of(RESULT)
        print(f"{label} shape with a stick -> {json.dumps(result)}")
        if result and result.get("item"):
            fail(f"the {label} stick control crafted {result['item']} "
                 f"x{result.get('count')}")
        run("close")
        wait(2)
    run("command", "gamemode creative @s")

    if failures:
        raise SystemExit(1)
    print("recipe PASS: every carried recipe — the eight material recipes, the three machines, "
          "the basic cell, and slice 5's orb, seven rods, six capacitors, five cells and the "
          "blazing crystal block both ways — crafts its declared output id and count, every "
          "ingredient is consumed, the lava bucket leaves its bucket, and both off-pattern grids "
          "leave the result empty")


if __name__ == "__main__":
    main()
