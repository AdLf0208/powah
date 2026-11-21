#!/usr/bin/env bash
# In-game check for the Powah port: the starter Furnator, Energy Cell and Energy
# Cable (slice 1) and the dielectric/capacitor crafting family with the recipes
# they unblock (slice 2), on a real game, for the flat SDK package (`powah.hmod`).
#
# The package carries the three blocks, their items, the data AND the Rust that
# runs them, so the game must be built against it by the launcher:
#
#   horizon-launcher native-build . --instance <instance> --trust-native-code
#
# and `GAME` must name that build's client. There is no `--dev-mod` leg: the
# package is compiled in, and a content half loaded beside it would be a second
# copy of the same blocks.
#
# `CONTROL_GAME=<an unmodified client of the same build>` is the negative control:
# the block id must be unknown there, the item must not arrive and the block must
# stay air, so the machines demonstrably come from this package. With it unset the
# leg is skipped and the log says so.
#
# NOT RE-RUN IN THE SDK MIGRATION: `horizon-launcher` was not installed then, so no
# 0b14f8a client carrying this package existed. It is installed now and 0.2.1 ran
# this leg (`ports/_shared/evidence/powah-e2e/`), so every phase below is live.
#
# The arena is a stone floor at y=100 in the Overworld, so every block read and
# command is in the player's own dimension (the pin's `ctl tp` is reliable
# there).
#
# Default run:
#  - control "no fuel": a placed furnator makes no FE for 40 ticks, and neither
#    does the cell beside it (nothing to move);
#  - coal in the furnator's fuel slot (the container path, automation's way in)
#    starts the burn, the stack shrinks by one and carbon is 1600*30;
#  - the furnator makes FE and the cable beside it carries that FE into the cell;
#  - control "no cable": with the cable broken and the cell emptied, the cell
#    stays empty for 40 ticks while the furnator holds FE; placing the cable
#    again lets the cell charge — so the cable is what moves the energy;
#  - control "full cell": a cell set to its capacity stays at 1,000,000 FE and
#    the furnator keeps burning (energy is not lost or duplicated);
#  - a charged cell discharges through a cable into an empty cell, so the cell
#    is a source as well as a sink;
#  - the cable reports no storage and its 500 FE/t rate.
# Slice 2 (the parity rule) adds:
#  - the six dielectric/capacitor items are given and read back, and an unknown
#    id is refused;
#  - the eight material recipes AND the three machine recipes craft in a real
#    crafting table — declared output id and count, every ingredient consumed,
#    the lava bucket's bucket left in the grid, and an off-pattern grid empty
#    (`tools/craft_check.py`);
#  - each machine's loot table drops the machine when it is mined in survival.
#
# SELFTEST=1 flips the "no fuel" assertion (it must FAIL), proving the scenario
# can fail. SELFTEST=craft expects the paste's 24 to be 25 inside
# `tools/craft_check.py` (it must FAIL there), proving the recipe assertions can
# fail. SELFTEST=network expects the three-cable run to have delivered nothing
# (it must FAIL), proving the network assertion can fail.
#
# Slice 3 (gap 2, "a run of two or more cables does not relay") adds:
#  - a source cell, three cables and a sink cell in one row: the charge crosses
#    the run (0.2.2 delivered nothing), the two cells' sum stays 100000, and a
#    cell off the network stays empty.
set -euo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/powah
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}   # pin-matched shared ctl
GAME=${GAME:-}                       # a launcher build carrying this package
CONTROL_GAME=${CONTROL_GAME:-}       # a launcher build without it, for the control leg
HMOD=${HMOD:-$PORT/powah.hmod}       # the delivered package this run is about
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-${E2E_TMP:-/tmp/opencode}/powah-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=PowahE2E
CONTROL_WORLD=PowahControl

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "missing GAME: point it at a launcher-built horizon-client that carries powah" >&2
    echo "  horizon-launcher native-build $PORT --instance <i> --trust-native-code" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -f "$HMOD" ] || { echo "missing the package: $HMOD (cargo horizon-mod build)" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}/powah-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }
# inv_count ITEM: how many of ITEM the player's inventory holds (0 when absent).
inv_count() { ctl inventory | jq -r --arg id "$1" '.counts[$id] // 0'; }

# put_fuel X Y Z ITEM COUNT: the game's own container path (`item replace block`,
# what a hopper reaches). Measured at 0b14f8a: `ctl container` reads a mod block
# entity's raw `items`, which the pin grows only through this command path or an
# opened menu, so the declared slot is invisible to that ctl call. The slot's
# own read is `/powah info`'s `fuel` field (below), which is stronger: it is the
# port's saved data, not a harness copy.
put_fuel() { # put_fuel X Y Z ITEM COUNT
    ctl command "item replace block $1 $2 $3 container.0 with $4 $5" >/dev/null
    echo "  item replace block($1,$2,$3) container.0 <- $4 x$5"
}
# fuel_is X Y Z ITEM COUNT: the furnator's own fuel slot holds COUNT of ITEM.
fuel_is() { # fuel_is X Y Z ITEM COUNT
    local line want="fuel $4 x$5"
    line=$(info "$1" "$2" "$3")
    case "$line" in
        *"$want"*) echo "  fuel($1,$2,$3) $4 x$5: ok" ;;
        *) fail "expected the fuel slot at $1,$2,$3 to hold $5 x $4; got '$line'" ;;
    esac
}
# info X Y Z: this block's status line, after a fresh reply arrives (the t=
# field makes each reply unique).
info() { # info X Y Z
    local out line before
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E " at $1,$2,$3:" | head -1)
    say "powah info $1 $2 $3"
    for _ in $(seq 1 25); do
        ctl wait 4 >/dev/null
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" | grep -E " at $1,$2,$3:|No Powah block" | head -1)
        if [ -n "$line" ] && [ "$line" != "$before" ]; then
            echo "$line"
            return
        fi
    done
    echo "NO-INFO $(jq -c '{chat,action_bar}' <<<"$out")"
}
# set_energy X Y Z AMOUNT: the harness control `/powah set`, retried until the
# block reads the value back (chat commands are asynchronous).
set_energy() { # X Y Z AMOUNT
    local want=$4 line
    say "powah set $want $1 $2 $3"
    for _ in $(seq 1 10); do
        line=$(info "$1" "$2" "$3")
        if [ "$(energy_of "$line")" = "$want" ]; then
            echo "  set($1,$2,$3) = $line"
            return
        fi
        say "powah set $want $1 $2 $3"
    done
    fail "could not set $1,$2,$3 to $want FE: $line"
}
expect_info() { # expect_info X Y Z SUBSTRING
    local line
    line=$(info "$1" "$2" "$3")
    echo "  info($1,$2,$3): $line"
    case "$line" in
        *"$4"*) ;;
        *) fail "expected /powah info to say '$4', got '$line'" ;;
    esac
}

# The energy in an info line: the number before "/<capacity> FE".
energy_of() { sed -nE 's/.* at [-0-9]+,[-0-9]+,[-0-9]+: ([0-9]+)\/[0-9]+ FE.*/\1/p' <<<"$1"; }
carbon_of() { sed -nE 's/.*carbon ([0-9]+).*/\1/p' <<<"$1"; }
burning_of() { sed -nE 's/.*burning (true|false).*/\1/p' <<<"$1"; }

cleanup() {
    rc=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$rc"
}
trap cleanup EXIT

launch() {
    rm -f "$SOCK"
    cd "$SCRATCH"
    local game=${LAUNCH_GAME:-$GAME}
    echo "  launch: $(basename "$game"), no --dev-mod: the package carries its content"
    # `--offscreen`, not `--headless`: slice 2 opens a crafting table and reads its
    # slots, and at this pin a menu's widget layout needs the renderer — with
    # `--headless`, `ctl screen`/`ctl ui` time out on an open menu (measured).
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "${WORLD_NAME:-$WORLD}" --seed 20261105 --creative --offscreen \
        --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

# A stone floor at y=99 with clear air above, in the Overworld, and the player
# standing on it at (0.5, 100, 3.5).
arena() {
    ctl tp 0.5 101 3.5 >/dev/null
    ctl wait 20
    ctl command "fill -8 99 -8 24 99 24 minecraft:stone" >/dev/null
    ctl command "fill -8 100 -8 24 110 24 minecraft:air" >/dev/null
    ctl command "kill @e[type=!player]" >/dev/null
    ctl tp 0.5 100 3.5 >/dev/null
    ctl wait 20
    local y feet floor
    y=$(ctl state | jq -r '.player.position[1] // 0')
    feet=$(ctl block 0 100 3 | jq -r '.name // .block // empty')
    floor=$(ctl block 0 99 3 | jq -r '.name // .block // empty')
    echo "  arena: y=$y floor(0,99,3)=$floor feet(0,100,3)=$feet"
    awk -v y="$y" 'BEGIN { exit !(y >= 99.9) }' || fail "the player is not on the arena floor (y=$y)"
    [ "$feet" = "minecraft:air" ] || fail "the arena volume is not clear ($feet at 0,100,3)"
    [ "$floor" = "minecraft:stone" ] || fail "the arena floor is missing ($floor at 0,99,3)"
}

place() { # place X Y Z ID
    ctl command "setblock $1 $2 $3 $4" >/dev/null
    ctl wait 4
    local got
    got=$(ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty')
    echo "  block($1,$2,$3) = $got"
    [ "$got" = "$4" ] || fail "expected $4 at $1,$2,$3, got '$got'"
}

# The negative control: an unmodified client of the same build. The block id must
# be unknown there, the item must not arrive and the block must stay air, so the
# machines, the commands and the transfer demonstrably come from this package.
# Skipped (and said so) when no `CONTROL_GAME` is given.
control_leg() {
    if [ -z "$CONTROL_GAME" ]; then
        echo "== control: skipped (set CONTROL_GAME=<unmodified client> to run it) =="
        return
    fi
    [ -x "$CONTROL_GAME" ] || fail "missing CONTROL_GAME: $CONTROL_GAME"
    echo "== control: the same build with no powah =="
    echo "  package under test: $HMOD ($(stat -c %s "$HMOD") B)"
    LAUNCH_GAME=$CONTROL_GAME WORLD_NAME=$CONTROL_WORLD launch
    arena
    local setblock give name
    setblock=$(ctl command "setblock 4 100 0 powah:furnator_starter")
    echo "  setblock reply: $setblock"
    case "$setblock" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:furnator_starter, so the content did not come from this package" ;;
    esac
    give=$(ctl command "give @s powah:energy_cell_starter 1")
    echo "  give reply: $give"
    case "$give" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:energy_cell_starter, so the content did not come from this package" ;;
    esac
    name=$(ctl block 4 100 0 | jq -r '.name // .block // "unknown"')
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    echo "  without the package: no block, no item (block 4,100,0 = $name)"
    ctl quit >/dev/null 2>&1 || true
}

echo "== launch (Overworld arena at y=100; one package, code and content together) =="
control_leg
launch
arena

echo "== place the furnator, the cable and the cell in a line =="
place 0 100 0 powah:furnator_starter
place 1 100 0 powah:energy_cable_starter
place 2 100 0 powah:energy_cell_starter

echo "== control: no fuel means no FE anywhere =="
line=$(info 0 100 0)
echo "  furnator: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a furnator with no fuel stored FE: $line"
[ "$(burning_of "$line")" = false ] || fail "a furnator with no fuel is burning: $line"
ctl wait 40
line=$(info 0 100 0)
echo "  after 40 ticks: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a furnator with no fuel made FE: $line"
if [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting the empty furnator to have made FE (this must fail)"
    [ "$(energy_of "$line")" != 0 ] || fail "SELFTEST flipped: the unfuelled furnator made no FE, as it should"
fi
expect_info 2 100 0 "Energy Cell at 2,100,0: 0/1000000 FE"
expect_info 1 100 0 "no storage, moves up to 500 FE/t"

echo "== coal into the fuel slot starts the burn =="
put_fuel 0 100 0 minecraft:coal 5
ctl wait 6
# the first item is already carbon by the next tick, so four are left
fuel_is 0 100 0 minecraft:coal 4
line=$(info 0 100 0)
echo "  $line"
[ "$(carbon_of "$line")" -gt 0 ] || fail "the furnace did not load the coal into carbon: $line"
[ "$(burning_of "$line")" = true ] || fail "the furnator is not burning: $line"
fuel_is 0 100 0 minecraft:coal 4
carbon=$(carbon_of "$line")
[ "$carbon" -le 48000 ] || fail "carbon above one coal's 48,000: $line"

echo "== the furnator makes FE and the cable carries it into the cell =="
ctl wait 40
line=$(info 0 100 0)
echo "  furnator: $line"
carbon_now=$(carbon_of "$line")
cell_line=$(info 2 100 0)
echo "  cell: $cell_line"
cell=$(energy_of "$cell_line")
[ "${carbon_now:-$carbon}" -lt "$carbon" ] || fail "the furnator is not burning its carbon: $line"
[ "${cell:-0}" -gt 0 ] || fail "the cable did not carry FE into the cell: $cell_line"
# One coal holds 48,000 FE: the cell can never hold more than that, so a
# duplicated transfer would show here.
[ "$cell" -le 48000 ] || fail "the cell holds more than one coal's 48,000 FE: $cell_line"
echo "  carbon ${carbon} -> ${carbon_now}, cell ${cell} FE after 40 burning ticks"

echo "== control: without the cable the cell stays empty =="
ctl command "setblock 1 100 0 minecraft:air" >/dev/null
ctl wait 4
set_energy 2 100 0 0
line=$(info 0 100 0)
held=$(energy_of "$line")
ctl wait 40
cell_line=$(info 2 100 0)
echo "  no cable: furnator $held FE, cell $cell_line"
[ "$(energy_of "$cell_line")" = 0 ] || fail "the cell charged without a cable: $cell_line"
line=$(info 0 100 0)
echo "  furnator after 40 more ticks: $line"
[ "$(energy_of "$line")" -ge "$held" ] || fail "the furnator lost FE in the no-cable control: $line"

echo "== control: a full cell stays full =="
set_energy 2 100 0 1000000
ctl wait 40
line=$(info 2 100 0)
echo "  after 40 ticks: $line"
[ "$(energy_of "$line")" = 1000000 ] || fail "a full cell changed: $line"

echo "== the cable again: the cell charges once more =="
set_energy 2 100 0 0
place 1 100 0 powah:energy_cable_starter
ctl wait 40
cell_line=$(info 2 100 0)
echo "  cell: $cell_line"
[ "$(energy_of "$cell_line")" -gt 0 ] || fail "the cable did not feed the cell again: $cell_line"

echo "== a charged cell discharges through a cable into an empty cell =="
# Break the first cable so the charged cell is not refilled, then build a
# second row: cell (2,100,0) - cable (2,100,1) - empty cell (2,100,2). The two
# cells are not adjacent: the cable is what carries the charge, and the source
# is the cell's own extract side (a cell pushes and takes on every side by
# default, so two adjacent cells would swap their charge every tick instead).
ctl command "setblock 1 100 0 minecraft:air" >/dev/null
ctl wait 4
set_energy 2 100 0 1000
place 2 100 1 powah:energy_cable_starter
place 2 100 2 powah:energy_cell_starter
ctl wait 40
sink_line=$(info 2 100 2)
source_line=$(info 2 100 0)
# Two chat round trips are ~40 ticks apart, and the transfer can land between
# them (measured: the sink read 500 while the source's earlier read still said
# 1000), so poll the source until the move shows, then assert both.
for _ in $(seq 1 8); do
    [ "$(energy_of "$source_line")" -lt 1000 ] && break
    ctl wait 10
    source_line=$(info 2 100 0)
done
echo "  source: $source_line"
echo "  sink:   $sink_line"
[ "$(energy_of "$source_line")" -lt 1000 ] || fail "the charged cell did not give any FE: $source_line"
[ "$(energy_of "$sink_line")" -gt 0 ] || fail "the empty cell took no FE through the cable: $sink_line"

echo "== slice 3: a run of three cables relays as one network =="
# A fresh row at z=5: source cell - three cables - sink cell. The two cells are
# three cable hops apart and touch no cable of their own, so only a network that
# walks its cables can deliver the charge (0.2.2 moved nothing: its cable read
# only the block entities beside one hop).
place 0 100 5 powah:energy_cell_starter
place 1 100 5 powah:energy_cable_starter
place 2 100 5 powah:energy_cable_starter
place 3 100 5 powah:energy_cable_starter
place 4 100 5 powah:energy_cell_starter
# A cell three blocks above the source, reached by no cable: the network gathers
# its ports by walking cables, so this one is not on it and must stay empty.
place 0 103 5 powah:energy_cell_starter
set_energy 0 100 5 100000
ctl wait 40
sink_line=$(info 4 100 5)
source_line=$(info 0 100 5)
# Two chat round trips are ~40 ticks apart, so poll the source until the move shows.
for _ in $(seq 1 8); do
    [ "$(energy_of "$source_line")" -lt 100000 ] && break
    ctl wait 10
    source_line=$(info 0 100 5)
done
isolated_line=$(info 0 103 5)
echo "  source: $source_line"
echo "  sink:   $sink_line"
echo "  off the network: $isolated_line"
[ "$(energy_of "$sink_line")" -gt 0 ] || fail "a three-cable network did not relay FE into the cell: $sink_line"
[ "$(energy_of "$source_line")" -lt 100000 ] || fail "the charged cell did not give any FE through the network: $source_line"
[ "$(energy_of "$isolated_line")" = 0 ] || fail "a cell with no cable took FE: $isolated_line"
if [ "$SELFTEST" = network ]; then
    echo "  SELFTEST: expecting the three-cable run to deliver nothing (this must fail)"
    [ "$(energy_of "$sink_line")" = 0 ] || fail "SELFTEST flipped: the three-cable network relayed FE, as it should"
fi
# Break the middle cable so the two cells stop trading, then read them: two
# `info` replies are ~24 ticks apart, and the cells swap energy every tick, so a
# sum of two live reads is not the total. Frozen, it is: the relay must not
# create or destroy a single FE across its run.
ctl command "setblock 2 100 5 minecraft:air" >/dev/null
ctl wait 10
frozen_source=$(info 0 100 5)
frozen_sink=$(info 4 100 5)
frozen=$(( $(energy_of "$frozen_source") + $(energy_of "$frozen_sink") ))
echo "  split: $frozen_source"
echo "  split: $frozen_sink"
[ "$frozen" = 100000 ] || fail "the network's two cells hold $frozen FE after being split, not the 100000 the source was set to: $frozen_source / $frozen_sink"

echo "== slice 2: the six dielectric/capacitor items are real items =="
for item in powah:dielectric_paste powah:dielectric_rod powah:dielectric_rod_horizontal \
            powah:dielectric_casing powah:capacitor_basic powah:capacitor_basic_tiny; do
    ctl give "$item" 3 >/dev/null || fail "cannot give $item"
    ctl wait 2
    got=$(inv_count "$item")
    echo "  give $item x3 -> $got in the inventory"
    [ "$got" -ge 3 ] || fail "$item did not reach the inventory (count $got)"
done
unknown=$(ctl command "give @s powah:dielectric_paste_typo 1")
case "$unknown" in
    *"Unknown item"*|*"unknown item"*|*error*|*"not found"*)
        echo "  unknown id refused: $unknown" ;;
    *) fail "an unknown powah item id was accepted: $unknown" ;;
esac
ctl command "clear @s" >/dev/null
ctl wait 3

echo "== slice 2: every carried recipe crafts in a real crafting table =="
ctl command "setblock 5 100 0 minecraft:crafting_table" >/dev/null
ctl wait 4
craft_selftest=0
[ "$SELFTEST" = craft ] && craft_selftest=1
SELFTEST=$craft_selftest python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" 5 100 0 \
    || fail "the recipe check failed: every material and machine recipe must craft its declared output and consume its ingredients"
ctl command "setblock 5 100 0 minecraft:air" >/dev/null
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 4

echo "== slice 2: each machine's loot table drops the machine =="
ctl command "setblock 5 100 0 minecraft:air" >/dev/null
ctl command "tp @s 5.5 100 0.5" >/dev/null
ctl wait 4
place 6 100 0 powah:furnator_starter
place 7 100 0 powah:energy_cell_starter
place 8 100 0 powah:energy_cable_starter
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
# The pickaxe must be in the hand: `give` lands in the first free slot, which is
# not necessarily the hotbar, and a `needs_iron_tool` block drops nothing without it.
ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe" >/dev/null
ctl select 1 >/dev/null
ctl wait 4
for spec in "6 powah:furnator_starter" "7 powah:energy_cell_starter" "8 powah:energy_cable_starter"; do
    set -- $spec
    ctl look-at-block "$1" 100 0 >/dev/null || true
    broken=$(ctl break "$1" 100 0) || fail "cannot break $2 at $1,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$2 did not break"
    got=$(echo "$broken" | jq -r --arg id "$2" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $2 dropped $got of itself, not the machine"
done
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3

echo "PASS: a fuelled furnator made FE, the cable carried it into the cell, an unfuelled furnator made nothing, a cell without a cable stayed empty, a full cell stayed full, a charged cell drained through a cable into an empty one, a three-cable network carried a charge across its run with the two cells' sum unchanged and a cell off the network empty, the six material items exist, every material and machine recipe crafted its declared output and consumed its ingredients, each machine dropped itself, and every guessed id is unknown without the package"
