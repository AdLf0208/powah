#!/usr/bin/env bash
# Proves the built content package (content/powah.hmod) loads on a real game:
# the shared, unmodded 1d63e0a5 game with `--dev-mod <the .hmod>` must register
# the three blocks and their items. A second launch WITHOUT `--dev-mod` is the
# negative control: the same ids must be unknown there, so the check cannot
# pass vacuously.
set -euo pipefail
export LC_ALL=C

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PIN_BIN/horizon-client}
HMOD=${HMOD:-$CLONE/ports/powah/content/powah.hmod}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/powah-hmod.sock}

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -f "$HMOD" ] || { echo "missing package: $HMOD" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/powah-hmod-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

cleanup() {
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "${KEEP:-0}" = 1 ]; then echo "kept scratch: $SCRATCH" >&2; else rm -rf "$SCRATCH" 2>/dev/null || true; fi
}
trap cleanup EXIT

launch() { # launch WORLD [--dev-mod PATH]
    local world=$1; shift
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261105 --creative --headless --wait \
        --resources "$RES" "$@" >/dev/null
    ctl wait-ready 120
    ctl wait 20
}

echo "== launch WITH the package (--dev-mod $HMOD) =="
launch PowahHmod --dev-mod "$HMOD"
ctl command "execute in minecraft:the_end run tp @s 0.5 64 3.5" >/dev/null
ctl wait 10
ctl command "execute in minecraft:the_end run setblock 0 64 0 powah:furnator_starter" >/dev/null
ctl command "execute in minecraft:the_end run setblock 2 64 0 powah:energy_cell_starter" >/dev/null
ctl command "execute in minecraft:the_end run setblock 4 64 0 powah:energy_cable_starter" >/dev/null
ctl wait 4
for pos in "0 64 0 powah:furnator_starter" "2 64 0 powah:energy_cell_starter" "4 64 0 powah:energy_cable_starter"; do
    set -- $pos
    block=$(ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty')
    echo "block $1,$2,$3 = $block"
    [ "$block" = "$4" ] || fail "the package did not register $4"
done
for item in powah:furnator_starter powah:energy_cell_starter powah:energy_cable_starter; do
    ctl give "$item" 1 >/dev/null
done
ctl wait 6
inventory=$(ctl inventory)
for item in powah:furnator_starter powah:energy_cell_starter powah:energy_cable_starter; do
    jq -e --arg item "$item" \
        '[.hotbar[] | select(.item == $item)] | length >= 1' <<<"$inventory" >/dev/null \
        || fail "the package did not register the $item item"
done
echo "items: $(jq -c '[.hotbar[] | select((.item // "") | startswith("powah:")) | {item, count}]' <<<"$inventory")"
ctl quit >/dev/null 2>&1 || true
sleep 2

echo "== negative control: the SAME ids without --dev-mod =="
launch PowahHmodControl
ctl command "execute in minecraft:the_end run tp @s 0.5 64 3.5" >/dev/null
ctl wait 10
out=$(ctl command "execute in minecraft:the_end run setblock 0 64 0 powah:furnator_starter")
echo "setblock reply: $out"
if grep -q 'Changed the block' <<<"$out"; then
    fail "the unmodded game accepted powah:furnator_starter (the check is vacuous)"
fi
block=$(ctl block 0 64 0 | jq -r '.name // .block // empty')
echo "control block 0,64,0 = $block"
[ "$block" != "powah:furnator_starter" ] || fail "the control block is the furnator"
ctl give powah:energy_cell_starter 1 >/dev/null || true
ctl wait 4
ctl inventory | jq -e '[.hotbar[] | select(.item == "powah:energy_cell_starter")] | length == 0' >/dev/null \
    || fail "the unmodded game gave a Powah energy cell item"

echo "PASS: powah.hmod registered the furnator, the energy cell and the energy cable (blocks + items) on a real game, and the same ids are unknown without the package"
echo "EXIT:0"
