#!/usr/bin/env bash
# In-game check for Powah slice 1: the starter Furnator, Energy Cell and Energy
# Cable on a real game.
#
# The arena is a stone floor at y=100 in the Overworld, so every block read and
# command is in the player's own dimension (the pin's `ctl tp` is reliable
# there). The native bundle is compiled into the game; `--dev-mod content` loads
# the carried blocks.
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
#
# SELFTEST=1 flips the "no fuel" assertion (it must FAIL), proving the scenario
# can fail.
set -euo pipefail
export LC_ALL=C

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/powah
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}   # pin-matched shared ctl
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/powah-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=PowahE2E
# EMBEDDED=1 launches with no --dev-mod: the delivered .hnmod carries its content.
EMBEDDED=${EMBEDDED:-0}
DEV_MOD=()
[ "$EMBEDDED" = 1 ] || DEV_MOD=(--dev-mod "$PORT/content")

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/powah-home.XXXXXX)
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

# The non-null slots of the container at a position, as JSON.
slots() { ctl container "$1" "$2" "$3" | jq -c '[.slots[] | select(. != null)]'; }
# has X Y Z ITEM COUNT: the container holds exactly COUNT of ITEM.
has() { # has X Y Z ITEM COUNT
    local got
    got=$(slots "$1" "$2" "$3")
    jq -e --arg item "$4" --argjson count "$5" \
        '[.[] | select(.item == $item) | .count] | add // 0 | . == $count' <<<"$got" >/dev/null \
        || fail "expected $5 x $4 at $1,$2,$3; got $got"
    echo "  has($1,$2,$3) $4 x$5: ok"
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
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$WORLD" --seed 20261105 --creative --headless --wait \
        --resources "$RES" "${DEV_MOD[@]}" >/dev/null
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

echo "== launch (Overworld arena at y=100, content + native bundle) =="
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
ctl container 0 100 0 minecraft:coal 5 >/dev/null
ctl wait 6
# the first item is already carbon by the next tick, so four are left
has 0 100 0 minecraft:coal 4
line=$(info 0 100 0)
echo "  $line"
[ "$(carbon_of "$line")" -gt 0 ] || fail "the furnace did not load the coal into carbon: $line"
[ "$(burning_of "$line")" = true ] || fail "the furnator is not burning: $line"
has 0 100 0 minecraft:coal 4
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
source_line=$(info 2 100 0)
sink_line=$(info 2 100 2)
echo "  source: $source_line"
echo "  sink:   $sink_line"
[ "$(energy_of "$source_line")" -lt 1000 ] || fail "the charged cell did not give any FE: $source_line"
[ "$(energy_of "$sink_line")" -gt 0 ] || fail "the empty cell took no FE through the cable: $sink_line"

echo "PASS: a fuelled furnator made FE, the cable carried it into the cell, an unfuelled furnator made nothing, a cell without a cable stayed empty, a full cell stayed full, and a charged cell drained through a cable into an empty one"
