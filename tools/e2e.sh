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
# Slice 5 (the five cell recipes' `blocked(energizing-orb)` gate) adds the
# Energizing Orb, its seven rods and the five materials they make:
#  - a starter rod beside an orb holding iron and gold fills it to exactly its
#    recipe's 10,000 FE and stands at 0, the orb writes 2 energized steel into
#    slot 0 and clears its inputs;
#  - an orb with no rod in range produces nothing, and a rod beside an orb whose
#    inputs match no recipe keeps its charge (the rod only gives to an orb that
#    `containRecipe()`s);
#  - a nitro rod fills four blaze powder to exactly 120,000 FE — it keeps the
#    other 80,000 — then the blaze-rod variant, a diamond (300,000), an emerald
#    (1,000,000) and the nether-star recipe (20,000,000 into 16 nitro crystals);
#  - the orb, every rod and the material block are given/placed/mined, and the
#    crafting table crafts the orb, the seven rods, the six capacitors and the
#    five cell recipes that were blocked before this slice.
# SELFTEST=orb expects the nitro rod to have kept its whole 200,000 (it must
# FAIL), proving the exact-fall assertion can fail.
#
# Slice 3 (gap 2, "a run of two or more cables does not relay") adds:
#  - a source cell, three cables and a sink cell in one row: the charge crosses
#    the run (0.2.2 delivered nothing), the two cells' sum stays 100000, and a
#    cell off the network stays empty.
#
# Slice 4 (gap 11, "the other 6 tiers ... are unported") adds the energy cell
# ladder:
#  - each of the seven tiers is placed and set to 2,000,000,000 FE; the cell
#    clamps it at its own tier's capacity (1M/4M/10M/40M/100M/400M/2G) and its
#    readout names its own per-side transfer;
#  - two adjacent pairs (starter source -> starter sink, basic source ->
#    starter sink) show the transfer in behaviour: the first swaps 1,000 each
#    way and its sink stays at 0..1,000, the second nets 3,000 a tick;
#  - each tier drops itself when mined in survival.
# SELFTEST=cells expects the basic cell to clamp at the starter's 1,000,000
# (it must FAIL there), proving the capacity assertion can fail.
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
# set_clamped X Y Z AMOUNT WANT: `/powah set AMOUNT`, retried until the cell reads
# WANT back. A tier cell clamps AMOUNT at its own capacity, so the reply's number
# is the clamp (not AMOUNT) and `set_energy`'s equality poll cannot be used.
set_clamped() { # X Y Z AMOUNT WANT
    local want=$5 line
    say "powah set $4 $1 $2 $3"
    for _ in $(seq 1 8); do
        line=$(info "$1" "$2" "$3")
        if [ "$(energy_of "$line")" = "$want" ]; then
            echo "  set($1,$2,$3) = $line"
            return
        fi
        say "powah set $4 $1 $2 $3"
    done
    fail "could not clamp $1,$2,$3 to $want FE: $line"
}
# set_floor X Y Z AMOUNT FLOOR: `/powah set AMOUNT`, retried until the cell holds
# at least FLOOR. A source beside a cable loses 500 FE/t as soon as it is set, so
# an exact-equality poll never settles (measured: 99,500 after one round trip);
# the phase's conservation check is what pins the total.
set_floor() { # X Y Z AMOUNT FLOOR
    local floor=$5 line
    say "powah set $4 $1 $2 $3"
    for _ in $(seq 1 10); do
        line=$(info "$1" "$2" "$3")
        if [ "$(energy_of "$line")" -ge "$floor" ]; then
            echo "  set($1,$2,$3) = $line"
            return
        fi
        say "powah set $4 $1 $2 $3"
    done
    fail "could not set $1,$2,$3 to at least $floor FE: $line"
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

# set_reply X Y Z AMOUNT SUBSTRING: `/powah set`, asserting the command's own
# reply. Used where the target feeds something and an equality poll on its buffer
# can never settle (a rod beside an orb with a recipe drains the same tick).
set_reply() { # X Y Z AMOUNT SUBSTRING
    local out line=""
    say "powah set $4 $1 $2 $3"
    # A chat command's reply lands a round trip later (~40 ticks), so the wait is
    # generous: ten iterations of `wait 4` read the chat before it arrived.
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        # `chat` is newest first, so the newest matching line is the first.
        line=$(jq -r '.chat[]' <<<"$out" |
            grep -E "Energy set to|stores no energy|No Powah block" | head -1)
        case "$line" in
            *"$5"*) echo "  set($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah set $4 at $1,$2,$3 to answer '$5', got '$line'"
}

# --- slice 5: the energizing orb's seven slots ---------------------------------

# orb_clear X Y Z: empty the orb's seven slots, the previous run's result included
# (the orb's tick clears them all and writes the result, as upstream's `inv.clear`
# does, so a stale result would be overwritten unreported).
orb_clear() { # orb_clear X Y Z
    local slot
    for slot in 0 1 2 3 4 5 6; do
        ctl command "item replace block $1 $2 $3 container.$slot with minecraft:air" >/dev/null
    done
    ctl wait 3
}
# orb_put X Y Z SLOT ITEM COUNT: one input stack, the way automation fills the orb
# (`item replace block ... container.N`, the same path the furnator's fuel uses).
orb_put() { # orb_put X Y Z SLOT ITEM COUNT
    ctl command "item replace block $1 $2 $3 container.$4 with $5 $6" >/dev/null
    ctl wait 2
}
# orb_slot X Y Z N: the orb's slot N, as `ctl container` reports it.
orb_slot() { # orb_slot X Y Z N
    ctl container "$1" "$2" "$3" | jq -c --argjson n "$4" '.slots[$n]'
}
# orb_wait X Y Z ITEM COUNT: poll slot 0 until it holds COUNT of ITEM. A chat round
# trip is ~40 ticks, and a nitro rod fills 200,000 a tick, so the wait is generous
# (a 20,000,000 FE recipe takes 100 fills).
orb_wait() { # orb_wait X Y Z ITEM COUNT
    local i slot
    for i in $(seq 1 40); do
        ctl wait 10
        slot=$(orb_slot "$1" "$2" "$3" 0)
        if [ "$(jq -r '.item // ""' <<<"$slot")" = "$4" ] &&
            [ "$(jq -r '.count // 0' <<<"$slot")" = "$5" ]; then
            echo "$slot"
            return 0
        fi
    done
    echo "$slot"
    return 1
}
# orb_no_result X Y Z: slot 0 is still empty (a control run).
orb_no_result() { # orb_no_result X Y Z
    local slot
    slot=$(orb_slot "$1" "$2" "$3" 0)
    [ "$(jq -r '.item // ""' <<<"$slot")" = "" ] ||
        fail "the orb at $1,$2,$3 produced $slot when it should not have"
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
    # Slice 4's ladder: the highest tier is unknown without the package too.
    tier=$(ctl command "setblock 5 100 0 powah:energy_cell_nitro")
    echo "  tier setblock reply: $tier"
    case "$tier" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energy_cell_nitro, so the content did not come from this package" ;;
    esac
    # Slice 5: the orb, a rod and the material it makes are all this package's.
    orb=$(ctl command "setblock 4 100 1 powah:energizing_orb")
    echo "  orb setblock reply: $orb"
    case "$orb" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energizing_orb, so the content did not come from this package" ;;
    esac
    rod=$(ctl command "setblock 4 100 2 powah:energizing_rod_nitro")
    echo "  rod setblock reply: $rod"
    case "$rod" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energizing_rod_nitro, so the content did not come from this package" ;;
    esac
    crystal=$(ctl command "give @s powah:crystal_blazing 1")
    echo "  crystal give reply: $crystal"
    case "$crystal" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:crystal_blazing, so the content did not come from this package" ;;
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
# The sink read can land before the move the source poll waited for (measured: sink
# 0 at t=552, source 500 at t=557), so re-read the sink until its charge shows.
for _ in $(seq 1 8); do
    [ "$(energy_of "$sink_line")" -gt 0 ] && break
    ctl wait 10
    sink_line=$(info 2 100 2)
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
set_floor 0 100 5 100000 1000
ctl wait 40
sink_line=$(info 4 100 5)
source_line=$(info 0 100 5)
# Two chat round trips are ~40 ticks apart, so the sink read can land before the
# relay's first move: poll the sink until the charge shows, then the source (its
# reading is a report, not an assertion — the relay pulls both ways).
for _ in $(seq 1 8); do
    [ "$(energy_of "$sink_line")" -gt 0 ] && break
    ctl wait 10
    sink_line=$(info 4 100 5)
done
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
# The source's live reading is not asserted: this relay pulls, so a charged sink
# gives its charge straight back through the same cables and the source reads
# 100000 +- 500 depending on which phase a chat round trip lands in (measured
# 99500 in the 0.2.3 run, 100500 in the next). The conservation check below
# freezes the transfer first, which is where "the charge moved and nothing was
# created" is proved.
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

echo "== slice 4: every energy cell tier holds its own capacity =="
# One command against every tier (`/powah set 2000000000 <pos>`): a cell clamps
# what it is given at its own tier's capacity, so the same command reads back a
# different exact number per tier (`DefaultEnergies`: baseScaling x 1,000,000).
tier_cell() { # X ID CAPACITY TRANSFER
    place "$1" 100 12 "$2"
    set_clamped "$1" 100 12 2000000000 "$3"
    local line expect=$3
    line=$(info "$1" 100 12)
    # SELFTEST=cells flips the basic tier's expectation to the starter's capacity;
    # the readout still says 4,000,000, so the assertion below must fail there.
    if [ "$SELFTEST" = cells ] && [ "$2" = powah:energy_cell_basic ]; then
        expect=1000000
        echo "  SELFTEST: expecting the basic cell to clamp at the starter's $expect (this must fail)"
    fi
    echo "  info($1,100,12): $line"
    case "$line" in
        *"$expect/$expect FE"*) ;;
        *) fail "expected $2 to hold $expect/$expect FE, got '$line'" ;;
    esac
    case "$line" in
        *"$4 FE/t a side"*) ;;
        *) fail "expected $2 to declare $4 FE/t a side, got '$line'" ;;
    esac
}
# The six tiers this slice adds first (the previous package does not know their
# ids at all), then the starter as the contrast it is clamped against.
tier_cell -4 powah:energy_cell_basic 4000000 4000
tier_cell -2 powah:energy_cell_hardened 10000000 10000
tier_cell 0 powah:energy_cell_blazing 40000000 40000
tier_cell 2 powah:energy_cell_niotic 100000000 100000
tier_cell 4 powah:energy_cell_spirited 400000000 400000
tier_cell 6 powah:energy_cell_nitro 2000000000 2000000
tier_cell -6 powah:energy_cell_starter 1000000 1000

echo "== slice 4: two adjacent cells swap and create nothing =="
# A cell gives its own transfer and accepts the neighbour's insert cap, and both
# give and take, so a pair nets zero by design (slice 1's deviation 3; upstream's
# answer is the side-config GUI, which is not ported). What the pair does prove,
# and what can fail: the starter sink never climbs past its own 1,000 insert cap
# beside a full basic cell (a wrong declaration would let it climb at 3,000 a
# tick), and the basic source never loses more than that cap — the pair swaps
# instead of creating or destroying FE.
place 16 100 16 powah:energy_cell_basic
place 16 100 17 powah:energy_cell_starter
say "powah set 4000000 16 100 16"
ctl wait 60
pair_source=$(info 16 100 16)
pair_sink=$(info 16 100 17)
echo "  pair source: $pair_source"
echo "  pair sink:   $pair_sink"
[ "$(energy_of "$pair_sink")" -le 1000 ] || fail "a starter sink beside a full basic cell holds $(energy_of "$pair_sink") FE; its insert cap is its own 1,000 a side, so the pair swaps and nothing nets: $pair_sink"
[ "$(energy_of "$pair_source")" -ge 3998000 ] || fail "the full basic cell lost $(energy_of "$pair_source") FE to an adjacent starter that gives its own 1,000 back: $pair_source"
ctl command "setblock 16 100 16 minecraft:air" >/dev/null
ctl wait 4

echo "== slice 5: the starter rod fills the orb with exactly the recipe's 10,000 FE =="
# An Energizing Orb holds seven slots (0 the result, 1-6 the inputs) and a buffer
# whose capacity is the recipe its inputs match: iron + gold is 10,000 FE into two
# energized steel. The starter rod holds exactly 10,000 and gives 100 FE/t, so the
# run is exact on both sides: the orb reaches its capacity and the rod reaches 0.
place 0 100 20 powah:energizing_orb
place 0 101 20 powah:energizing_rod_starter
orb_clear 0 100 20
orb_put 0 100 20 1 minecraft:iron_ingot 1
orb_put 0 100 20 2 minecraft:gold_ingot 1
expect_info 0 100 20 "0/10000 FE (recipe powah:steel_energized x2, inputs 2"
expect_info 0 101 20 "0/10000 FE (100 FE/t a side, orb 0,100,20"
# The starter rod's capacity is 10,000 (`energizingCapacity`), so `/powah set
# 20000` clamps, and the command says so: the first exact read of this slice's
# own numbers. (The rod drains while a reply lands, so the reply is the assertion.)
set_reply 0 101 20 20000 "Energy set to 10000 FE."
steel=$(orb_wait 0 100 20 powah:steel_energized 2) ||
    fail "the starter rod did not energize iron and gold into 2 energized steel: $steel / $(info 0 100 20) / $(info 0 101 20)"
echo "  output(0,100,20) = $steel"
steel_rod=$(info 0 101 20)
echo "  rod after: $steel_rod"
[ "$(energy_of "$steel_rod")" = 0 ] ||
    fail "the rod held 10,000 FE and gave 100 a tick; it should have given exactly 10,000 and stand at 0: $steel_rod"
expect_info 0 100 20 "0/0 FE (no recipe"
[ "$(orb_slot 0 100 20 1)" = "null" ] || fail "the orb kept its input slot 1: $(orb_slot 0 100 20 1)"
[ "$(orb_slot 0 100 20 2)" = "null" ] || fail "the orb kept its input slot 2: $(orb_slot 0 100 20 2)"

echo "== slice 5: an orb with no rod in range produces nothing =="
place 10 100 20 powah:energizing_orb
orb_clear 10 100 20
orb_put 10 100 20 1 minecraft:iron_ingot 1
orb_put 10 100 20 2 minecraft:gold_ingot 1
ctl wait 60
orb_no_result 10 100 20
expect_info 10 100 20 "0/10000 FE (recipe powah:steel_energized x2, inputs 2"

echo "== slice 5: a rod beside an orb with no recipe keeps its charge =="
# Upstream's `postTick` only fills while the orb `containRecipe()`s; an orb holding
# something no recipe matches is inert, and the rod must not leak into it.
place 20 100 20 powah:energizing_orb
place 20 101 20 powah:energizing_rod_nitro
orb_clear 20 100 20
orb_put 20 100 20 1 minecraft:stick 1
set_clamped 20 101 20 5000 5000
ctl wait 60
expect_info 20 101 20 "5000/20000000 FE"
expect_info 20 100 20 "0/0 FE (no recipe"
orb_no_result 20 100 20

echo "== slice 5: a nitro rod makes a blazing crystal at exactly 120,000 FE =="
# Four blaze powder at 120,000 FE. The nitro rod holds and gives 200,000, so the
# orb takes exactly its recipe's energy out of the rod and the rod is left with
# 80,000 — the assertion a wrong cap or a wrong move would break.
place 0 100 28 powah:energizing_orb
place 0 101 28 powah:energizing_rod_nitro
orb_clear 0 100 28
for slot in 1 2 3 4; do orb_put 0 100 28 "$slot" minecraft:blaze_powder 1; done
expect_info 0 100 28 "0/120000 FE (recipe powah:crystal_blazing x1, inputs 4"
set_reply 0 101 28 200000 "Energy set to 200000 FE."
blazing=$(orb_wait 0 100 28 powah:crystal_blazing 1) ||
    fail "the nitro rod did not energize four blaze powder into a blazing crystal: $blazing / $(info 0 100 28) / $(info 0 101 28)"
echo "  output(0,100,28) = $blazing"
rod_left=80000
if [ "$SELFTEST" = orb ]; then
    # The counterfactual: a run that claims the rod kept its whole charge must FAIL
    # here, so the assertion is alive.
    rod_left=200000
    echo "  SELFTEST: expecting the rod to have kept its whole 200,000 (this must fail)"
fi
expect_info 0 101 28 "$rod_left/20000000 FE"
expect_info 0 100 28 "0/0 FE (no recipe"
[ "$(orb_slot 0 100 28 4)" = "null" ] || fail "the orb kept its fourth input slot: $(orb_slot 0 100 28 4)"

echo "== slice 5: the jar's second blazing recipe, one blaze rod =="
orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:blaze_rod 1
set_reply 0 101 28 200000 "Energy set to 200000 FE."
rod_blaze=$(orb_wait 0 100 28 powah:crystal_blazing 1) ||
    fail "one blaze rod did not energize into a blazing crystal: $rod_blaze / $(info 0 100 28)"
echo "  output(0,100,28) = $rod_blaze"
expect_info 0 101 28 "80000/20000000 FE"

echo "== slice 5: the niotic, spirited and nitro crystals =="
orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:diamond 1
expect_info 0 100 28 "0/300000 FE (recipe powah:crystal_niotic x1, inputs 1"
set_reply 0 101 28 400000 "Energy set to 400000 FE."
niotic=$(orb_wait 0 100 28 powah:crystal_niotic 1) ||
    fail "a diamond did not energize into a niotic crystal: $niotic / $(info 0 100 28)"
echo "  output(0,100,28) = $niotic"
expect_info 0 101 28 "100000/20000000 FE"

orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:emerald 1
set_reply 0 101 28 1000000 "Energy set to 1000000 FE."
spirited=$(orb_wait 0 100 28 powah:crystal_spirited 1) ||
    fail "an emerald did not energize into a spirited crystal: $spirited / $(info 0 100 28)"
echo "  output(0,100,28) = $spirited"
expect_info 0 101 28 "0/20000000 FE"

orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:nether_star 1
orb_put 0 100 28 2 minecraft:redstone_block 1
orb_put 0 100 28 3 minecraft:redstone_block 1
orb_put 0 100 28 4 powah:blazing_crystal_block 1
expect_info 0 100 28 "0/20000000 FE (recipe powah:crystal_nitro x16, inputs 4"
set_reply 0 101 28 20000000 "Energy set to 20000000 FE."
# The largest recipe: 20,000,000 FE at 200,000 a tick is 100 fills after the
# rod's 20-tick warm-up, so this phase is the one that can time out.
nitro=$(orb_wait 0 100 28 powah:crystal_nitro 16) ||
    fail "the nitro recipe did not produce 16 nitro crystals: $nitro / $(info 0 100 28) / $(info 0 101 28)"
echo "  output(0,100,28) = $nitro"
expect_info 0 101 28 "0/20000000 FE"

echo "== slice 2: the six dielectric/capacitor items are real items =="
for item in powah:dielectric_paste powah:dielectric_rod powah:dielectric_rod_horizontal \
            powah:dielectric_casing powah:capacitor_basic powah:capacitor_basic_tiny \
            powah:steel_energized powah:crystal_blazing powah:crystal_niotic \
            powah:crystal_spirited powah:crystal_nitro powah:capacitor_basic_large \
            powah:capacitor_hardened powah:capacitor_blazing powah:capacitor_niotic \
            powah:capacitor_spirited powah:capacitor_nitro; do
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
unknown=$(ctl command "give @s powah:crystal_uraninite 1")
case "$unknown" in
    *"Unknown item"*|*"unknown item"*|*error*|*"not found"*)
        echo "  unknown material id refused: $unknown" ;;
    *) fail "an unknown powah material id was accepted: $unknown" ;;
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
# Slice 4: every cell tier drops itself too.
for tier in basic hardened blazing niotic spirited nitro; do
    place 6 100 0 "powah:energy_cell_$tier"
    ctl look-at-block 6 100 0 >/dev/null || true
    broken=$(ctl break 6 100 0) || fail "cannot break powah:energy_cell_$tier at 6,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "powah:energy_cell_$tier did not break"
    got=$(echo "$broken" | jq -r --arg id "powah:energy_cell_$tier" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking powah:energy_cell_$tier dropped $got of itself, not the machine"
done
# Slice 5: the orb, every rod tier and the material block drop themselves.
for block in powah:energizing_orb powah:energizing_rod_starter powah:energizing_rod_basic \
             powah:energizing_rod_hardened powah:energizing_rod_blazing \
             powah:energizing_rod_niotic powah:energizing_rod_spirited \
             powah:energizing_rod_nitro powah:blazing_crystal_block; do
    place 6 100 0 "$block"
    ctl look-at-block 6 100 0 >/dev/null || true
    broken=$(ctl break 6 100 0) || fail "cannot break $block at 6,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
    got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
done
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3

echo "PASS: a fuelled furnator made FE, the cable carried it into the cell, an unfuelled furnator made nothing, a cell without a cable stayed empty, a full cell stayed full, a charged cell drained through a cable into an empty one, a three-cable network carried a charge across its run with the two cells' sum unchanged and a cell off the network empty, the six material items exist, every material and machine recipe crafted its declared output and consumed its ingredients, each machine dropped itself, every energy cell tier clamped 2,000,000,000 FE at its own capacity and carried its own transfer, two adjacent cells swapped within their own transfer cap without creating FE, every cell tier dropped itself, a starter rod filled the orb to exactly its recipe's 10,000 FE and stood at 0, an orb with no rod and an orb with no recipe produced nothing while its rod kept its charge, a nitro rod made a blazing crystal at exactly 120,000 FE and kept the other 80,000, the blaze-rod blazing recipe, the niotic, spirited and nitro crystals all energised, every new block and item exists, every carried recipe including the five previously blocked cells crafts its declared output and consumes its ingredients, and every guessed id is unknown without the package"
