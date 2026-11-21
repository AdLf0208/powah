// Pure machine logic for Powah slice 1 (Furnator, Energy Cell, Energy Cable).
//
// This file is both the standalone Cargo library and the injected
// `crate::game::powah` module. It has no game types: the hook glue in
// `support.rs` turns block entities, energy buffers and item stacks into the
// small values used here.
//
// Numbers are upstream's STARTER-tier defaults (`config/v2/DefaultEnergies`):
// see `FINDINGS.md` for the derivation.

/// Furnator STARTER buffer (`generatorCapacity`).
pub const FURNATOR_CAPACITY: i32 = 20_000;
/// Furnator STARTER transfer, per side and per tick (`generatorTransfer`).
pub const FURNATOR_TRANSFER: i32 = 80;
/// Furnator generation while burning (`activeProduction`).
pub const FURNATOR_GENERATION: i32 = 20;
/// `general.energy_per_fuel_tick`: one burn tick is worth this much carbon.
pub const ENERGY_PER_FUEL_TICK: i32 = 30;
/// Energy Cell STARTER buffer (`batteryCapacity`).
pub const CELL_CAPACITY: i32 = 1_000_000;
/// Energy Cell STARTER transfer, per side and per tick (`batteryTransfer`).
pub const CELL_TRANSFER: i32 = 1_000;
/// Energy Cable STARTER transfer (`cableTransfer`); the cable stores nothing.
pub const CABLE_TRANSFER: i32 = 500;

/// The tiers a non-creative Powah block has (`Tier.getNormalVariants`), in order:
/// starter, basic, hardened, blazing, niotic, spirited, nitro.
pub const CELL_TIER_COUNT: usize = 7;

/// How far from an orb a rod may stand (`PowahConfig.general.energizing_range`),
/// in blocks, Chebyshev distance like upstream's `BlockPos.betweenClosed`.
pub const ENERGIZING_RANGE: i32 = 4;
/// The orb's inventory (`this.inv.set(7)`): slot 0 is the result, 1..6 the inputs.
pub const ORB_SLOTS: usize = 7;
/// The orb's result slot, the only one a hopper may take from.
pub const ORB_OUTPUT_SLOT: usize = 0;
/// The orb's six input slots, the only ones a hopper may fill.
pub const ORB_INPUT_SLOTS: usize = ORB_SLOTS - 1;
/// The orb's declared buffer: the largest energizing recipe
/// (`crystal_nitro`'s 20,000,000). A matched recipe's own energy is the cap its
/// tick enforces and its readout shows.
pub const ORB_MAX_ENERGY: i32 = 20_000_000;
/// How many ticks a rod charges before its first fill
/// (`EnergizingRodTile`'s `Ticker(20)`); it fills every tick after that.
pub const ROD_WARMUP: i32 = 20;

/// One energy cell tier's storage numbers: `batteryCapacity` and
/// `batteryTransfer` at that tier (`config/v2/DefaultEnergies`, both derived from
/// `baseScaling` — `× 1_000_000` and `× 1_000`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellTier {
    pub capacity: i32,
    pub transfer: i32,
}

/// The energy cell tier at `index` (0 = starter … 6 = nitro), `None` past the
/// ladder — the creative cell is a source, not a storage tier, and is not carried.
#[must_use]
pub fn cell_tier(index: usize) -> Option<CellTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; CELL_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `batteryCapacity` = `reactorProduction * 4000` = `baseScaling * 250 * 4000`.
    const CAPACITY_PER_SCALING: i32 = 1_000_000;
    /// `batteryTransfer` = `cableTransfer * 2` = `baseScaling * 250 * 4`.
    const TRANSFER_PER_SCALING: i32 = 1_000;
    let scaling = *BASE_SCALING.get(index)?;
    Some(CellTier {
        capacity: scaling * CAPACITY_PER_SCALING,
        transfer: scaling * TRANSFER_PER_SCALING,
    })
}

/// Energy clamped into a buffer.
#[must_use]
pub fn clamp_energy(energy: i32, capacity: i32) -> i32 {
    energy.clamp(0, capacity.max(0))
}

/// `floor(energy / capacity * 15)`, as upstream's `getComparatorStrength`.
#[must_use]
pub fn comparator_strength(energy: i32, capacity: i32) -> i32 {
    if capacity <= 0 {
        return 0;
    }
    let strength = (i64::from(energy.max(0)) * 15 / i64::from(capacity)) as i32;
    strength.clamp(0, 15)
}

/// Upstream's `extractFromSides`: at most the transfer, and never more than is
/// stored.
#[must_use]
pub fn push_amount(stored: i32, transfer: i32) -> i32 {
    stored.max(0).min(transfer.max(0))
}

/// The Furnator's saved state: the buffer and the carbon left to burn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Furnator {
    pub energy: i32,
    pub carbon: i32,
}

impl Furnator {
    /// The empty furnator.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, and the game build does not compile the tests"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream's refuel gate: no carbon left (the buffer's room does not
    /// matter — fuel becomes carbon first and burns when there is room).
    #[must_use]
    pub fn wants_fuel(&self) -> bool {
        self.carbon <= 0
    }

    /// True while carbon is left (upstream's `!carbon.isEmpty()`).
    #[must_use]
    pub fn burning(&self) -> bool {
        self.carbon > 0
    }

    /// Take one fuel item's burn time. False when nothing was taken.
    pub fn load_fuel(&mut self, burn_time: u32) -> bool {
        if self.burning() || burn_time == 0 {
            return false;
        }
        self.carbon = i32::try_from(burn_time)
            .unwrap_or(i32::MAX)
            .saturating_mul(ENERGY_PER_FUEL_TICK);
        true
    }

    /// One server tick: produce `min(space, generation, carbon)`. Returns the
    /// FE stored (0 when the buffer is full or nothing is burning). A full
    /// buffer does not drain carbon.
    pub fn tick(&mut self) -> i32 {
        let space = (FURNATOR_CAPACITY - self.energy).max(0);
        let produced = space.min(FURNATOR_GENERATION).min(self.carbon.max(0));
        if produced > 0 {
            self.energy += produced;
            self.carbon -= produced;
        }
        produced
    }
}

/// A neighbour's energy buffer as one side reaches it (`native_energy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Neighbour {
    pub energy: i32,
    pub capacity: i32,
    pub max_insert: i32,
    pub max_extract: i32,
    pub can_insert: bool,
    pub can_extract: bool,
}

impl Neighbour {
    /// The room left for an insert.
    #[must_use]
    pub fn space(&self) -> i32 {
        if self.can_insert {
            (self.capacity - self.energy).max(0)
        } else {
            0
        }
    }

    /// What one move may take out of it.
    #[must_use]
    pub fn available(&self) -> i32 {
        if self.can_extract {
            self.energy.max(0).min(self.max_extract.max(0))
        } else {
            0
        }
    }
}

/// One `native_move_energy` call: `from` and `to` are neighbour indexes
/// (0 = down, as `Direction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub from: usize,
    pub to: usize,
    pub amount: i32,
}

/// The moves one cable makes in a tick, from its six sides as `native_energy`
/// presents them: from each neighbour that can give into each that can take, up
/// to the cable's transfer rate in total. `rotation` rotates the starting side so
/// two equal sources share the cable as upstream's round-robin does. A cable
/// stores nothing, so no move starts and ends at the same neighbour, and the
/// total never exceeds `transfer`.
///
/// This is [`network_moves`] on one cable's six sides; both delegate to the one
/// planner, so the single-cable plan slice 1 proved in game and the network plan
/// cannot drift apart.
#[must_use]
pub fn cable_moves(
    neighbours: &[Option<Neighbour>; 6],
    transfer: i32,
    rotation: usize,
) -> Vec<Transfer> {
    plan(neighbours, transfer, rotation)
}

/// The one planner: `sides` is every place the relay can reach, in the order the
/// caller gathered them (`None` for an unreachable place). Upstream's
/// `CableNet` walks the connected cables round-robin and pushes into each energy
/// side at most a cable's transfer; here one plan covers the whole relay, so the
/// budget is spent once across every source and sink in it.
fn plan(sides: &[Option<Neighbour>], transfer: i32, rotation: usize) -> Vec<Transfer> {
    let mut moves = Vec::new();
    let count = sides.len();
    if count == 0 {
        return moves;
    }
    let start = rotation % count;
    let mut budget = transfer.max(0);
    let mut sink = sides.to_vec();
    for step in 0..count {
        if budget == 0 {
            break;
        }
        let from = (start + step) % count;
        let Some(source) = sides[from] else {
            continue;
        };
        let mut available = source.available();
        for other in 0..count {
            if available == 0 || budget == 0 {
                break;
            }
            let to = (start + other) % count;
            if to == from {
                continue;
            }
            let Some(target) = sink[to] else {
                continue;
            };
            let amount = available
                .min(target.space())
                .min(target.max_insert.max(0))
                .min(budget);
            if amount <= 0 {
                continue;
            }
            moves.push(Transfer { from, to, amount });
            available -= amount;
            budget -= amount;
            let mut after = target;
            after.energy += amount;
            sink[to] = Some(after);
        }
    }
    moves
}

/// The moves one **cable network** makes in a tick. `ports` are the network's
/// external energy sides — every neighbour that is not a cable, of every cable in
/// the network — in the order the caller gathered them; `from`/`to` index that
/// slice.
///
/// The network is one relay: it spends one cable's transfer per tick, not one per
/// cable, so a longer run carries the same rate and the readout's "moves up to
/// 500 FE/t" stays true for the whole network.
#[must_use]
pub fn network_moves(ports: &[Neighbour], transfer: i32, rotation: usize) -> Vec<Transfer> {
    let sides: Vec<Option<Neighbour>> = ports.iter().copied().map(Some).collect();
    plan(&sides, transfer, rotation)
}

/// The most ingredients one energizing recipe has (the nitro crystal's four).
pub const MAX_INGREDIENTS: usize = 6;

/// The tiers an energizing rod has, in `Tier.getNormalVariants` order: starter,
/// basic, hardened, blazing, niotic, spirited, nitro.
pub const ROD_TIER_COUNT: usize = 7;

/// One energizing rod tier's numbers: `energizingCapacity` and
/// `energizingTransfer` at that tier (`config/v2/DefaultEnergies`, both derived
/// from `baseScaling` — `× 10_000` and `× 100`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RodTier {
    pub capacity: i32,
    pub transfer: i32,
}

/// The energizing rod tier at `index` (0 = starter … 6 = nitro), `None` past the
/// ladder.
#[must_use]
pub fn rod_tier(index: usize) -> Option<RodTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; ROD_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `energizingTransfer` = `baseScaling * 100`.
    const TRANSFER_PER_SCALING: i32 = 100;
    /// `energizingCapacity` = `energizingTransfer * 100`.
    const CAPACITY_PER_SCALING: i32 = 10_000;
    let scaling = *BASE_SCALING.get(index)?;
    Some(RodTier {
        capacity: scaling * CAPACITY_PER_SCALING,
        transfer: scaling * TRANSFER_PER_SCALING,
    })
}

/// One energizing recipe, as the jar's `data/powah/recipe/energizing/*.json`
/// declare it: the result, the energy the orb's buffer must reach, and the
/// ingredients the six input slots must hold.
///
/// An ingredient is the list of item ids one jar ingredient accepts. The jar
/// writes most of them as NeoForge `c:` tags, which 26.3 has no equivalent of;
/// each is carried as the vanilla member(s) it resolves to (see `notes.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergizingRecipe {
    pub output: &'static str,
    pub count: i32,
    pub energy: i32,
    pub ingredients: &'static [&'static [&'static str]],
}

/// The energizing recipes this port carries, in the jar's own order: steel, the
/// one blaze-powder and the one blaze-rod blazing crystal, niotic, spirited and
/// the nitro crystal.
#[must_use]
pub fn energizing() -> &'static [EnergizingRecipe] {
    /// The jar's `data/powah/recipe/energizing/*.json`. Each jar ingredient is a
    /// NeoForge `c:` tag (or an item); 26.3 has no `c:` tags, so the list is the
    /// vanilla member(s) the tag resolves to (see `notes.md`, slice 5).
    static RECIPES: [EnergizingRecipe; 6] = [
        EnergizingRecipe {
            output: "powah:steel_energized",
            count: 2,
            energy: 10_000,
            // `c:ingots/iron`, `c:ingots/gold`
            ingredients: &[&["minecraft:iron_ingot"], &["minecraft:gold_ingot"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_blazing",
            count: 1,
            energy: 120_000,
            // four `minecraft:blaze_powder`
            ingredients: &[
                &["minecraft:blaze_powder"],
                &["minecraft:blaze_powder"],
                &["minecraft:blaze_powder"],
                &["minecraft:blaze_powder"],
            ],
        },
        EnergizingRecipe {
            output: "powah:crystal_blazing",
            count: 1,
            energy: 120_000,
            // `c:rods/blaze`
            ingredients: &[&["minecraft:blaze_rod"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_niotic",
            count: 1,
            energy: 300_000,
            // `c:gems/diamond`
            ingredients: &[&["minecraft:diamond"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_spirited",
            count: 1,
            energy: 1_000_000,
            // `c:gems/emerald`
            ingredients: &[&["minecraft:emerald"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_nitro",
            count: 16,
            energy: 20_000_000,
            // `c:nether_stars`, two `c:storage_blocks/redstone`, a blazing crystal block
            ingredients: &[
                &["minecraft:nether_star"],
                &["minecraft:redstone_block"],
                &["minecraft:redstone_block"],
                &["powah:blazing_crystal_block"],
            ],
        },
    ];
    &RECIPES
}

impl EnergizingRecipe {
    /// Upstream's `EnergizingRecipe.matches`: each filled input slot takes one
    /// ingredient, in any order, and every ingredient must be used. A stack of
    /// four blaze powder in one slot is *one* filled slot, as the jar's
    /// slot-limit-1 inputs make it.
    #[must_use]
    pub fn matches(&self, inputs: &[&str]) -> bool {
        let mut used = [false; MAX_INGREDIENTS];
        if inputs.len() != self.ingredients.len() || inputs.len() > MAX_INGREDIENTS {
            return false;
        }
        for input in inputs {
            let mut taken = false;
            for (index, ingredient) in self.ingredients.iter().enumerate() {
                if !used[index] && ingredient.contains(input) {
                    used[index] = true;
                    taken = true;
                    break;
                }
            }
            if !taken {
                return false;
            }
        }
        true
    }
}

/// The recipe the orb's filled input slots match, `None` when they match none.
#[must_use]
pub fn energize(inputs: &[&str]) -> Option<&'static EnergizingRecipe> {
    energizing().iter().find(|recipe| recipe.matches(inputs))
}

/// What one orb tick decides: the recipe its inputs match, the buffer it holds
/// afterwards (0 when it completes or clears) and whether the recipe is finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrbStep {
    pub recipe: Option<&'static EnergizingRecipe>,
    pub energy: i32,
    pub done: bool,
}

/// The orb's rule (`EnergizingOrbTile.onSlotChanged` + `fillEnergy`): a matching
/// recipe sets the buffer's capacity to *its* energy, a different recipe clears
/// it (`onSlotChanged` zeroes the buffer), and a buffer that reached the capacity
/// produces the result. `recorded` is the recipe output the orb last matched;
/// `None` adopts the energy a rod already moved in (upstream has no such window,
/// but a rod may fill on the tick the inputs arrive, and that energy is the
/// recipe's, not another's).
///
/// A buffer above the recipe's energy is `done`, never clamped down to it: the
/// card only ever takes room up to the capacity, so a higher value can only come
/// from a harness `set`.
#[must_use]
pub fn orb_step(inputs: &[&str], energy: i32, recorded: Option<&str>) -> OrbStep {
    let Some(recipe) = energize(inputs) else {
        return OrbStep {
            recipe: None,
            energy: 0,
            done: false,
        };
    };
    let changed_recipe = recorded.is_some_and(|output| output != recipe.output);
    let held = if changed_recipe {
        0
    } else {
        clamp_energy(energy, recipe.energy)
    };
    let done = held >= recipe.energy;
    OrbStep {
        recipe: Some(recipe),
        energy: if done { 0 } else { held },
        done,
    }
}

/// What one rod tick moves into its orb, from the orb's own formula
/// (`EnergizingOrbTile.fillEnergy`): the room left in the orb, capped by what the
/// rod holds and by the rod tier's transfer.
#[must_use]
pub fn orb_fill(rod_stored: i32, rod_transfer: i32, orb_stored: i32, capacity: i32) -> i32 {
    let room = (capacity.max(0) - orb_stored.max(0)).max(0);
    room.min(rod_stored.max(0).min(rod_transfer.max(0)))
}

/// One rod tick's warm-up (`EnergizingRodTile`'s `Ticker(20)`): `charging` is
/// true while the orb has a recipe *and* the rod holds energy. The counter walks
/// up to [`ROD_WARMUP`] and then stays there, which is upstream's behaviour: the
/// warm-up delays the first fill by 20 ticks, it does not throttle later ones.
#[must_use]
pub fn rod_cooldown(charging: bool, cooldown: i32) -> i32 {
    if charging {
        (cooldown.max(0) + 1).min(ROD_WARMUP)
    } else {
        (cooldown - 1).max(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(energy: i32, capacity: i32, max_insert: i32, max_extract: i32) -> Neighbour {
        Neighbour {
            energy,
            capacity,
            max_insert,
            max_extract,
            can_insert: max_insert > 0,
            can_extract: max_extract > 0,
        }
    }

    fn empty() -> [Option<Neighbour>; 6] {
        [None; 6]
    }

    #[test]
    fn every_energy_cell_tier_scales_upstreams_battery_numbers() {
        // `DefaultEnergies`: baseScaling = 1, 4, 10, 40, 100, 400, 2000;
        // batteryCapacity = reactorProduction * 4000 = baseScaling * 1_000_000 and
        // batteryTransfer = cableTransfer * 2 = reactorProduction * 4 = baseScaling * 1_000.
        let expected = [
            (1_000_000, 1_000),
            (4_000_000, 4_000),
            (10_000_000, 10_000),
            (40_000_000, 40_000),
            (100_000_000, 100_000),
            (400_000_000, 400_000),
            (2_000_000_000, 2_000_000),
        ];
        for (index, (capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = cell_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "tier {index}'s numbers"
            );
        }
        assert_eq!(
            cell_tier(CELL_TIER_COUNT),
            None,
            "the creative tier is not a storage tier"
        );
    }

    #[test]
    fn the_starter_cell_keeps_slice_1_s_numbers() {
        let starter = cell_tier(0).expect("the starter tier is the first of the ladder");
        assert_eq!(
            (starter.capacity, starter.transfer),
            (CELL_CAPACITY, CELL_TRANSFER)
        );
    }

    #[test]
    fn energy_is_clamped_into_the_buffer() {
        assert_eq!(clamp_energy(-5, 100), 0);
        assert_eq!(clamp_energy(50, 100), 50);
        assert_eq!(clamp_energy(500, 100), 100);
        assert_eq!(clamp_energy(5, 0), 0);
    }

    #[test]
    fn the_comparator_reads_the_fraction_of_the_buffer() {
        assert_eq!(comparator_strength(0, CELL_CAPACITY), 0);
        assert_eq!(comparator_strength(CELL_CAPACITY, CELL_CAPACITY), 15);
        assert_eq!(comparator_strength(CELL_CAPACITY / 2, CELL_CAPACITY), 7);
        assert_eq!(comparator_strength(5, 0), 0);
    }

    #[test]
    fn a_push_moves_at_most_the_transfer_and_never_more_than_is_stored() {
        assert_eq!(push_amount(10, 80), 10);
        assert_eq!(push_amount(1_000, 80), 80);
        assert_eq!(push_amount(0, 80), 0);
        assert_eq!(push_amount(-1, 80), 0);
    }

    #[test]
    fn a_furnator_with_no_fuel_makes_nothing() {
        let mut furnator = Furnator::new();
        assert_eq!(furnator.tick(), 0);
        assert_eq!(furnator.energy, 0);
        assert!(!furnator.burning());
    }

    #[test]
    fn one_coal_is_thirty_per_burn_tick_of_carbon() {
        let mut furnator = Furnator::new();
        assert!(furnator.load_fuel(1_600));
        assert_eq!(furnator.carbon, 48_000);
        assert_eq!(furnator.tick(), FURNATOR_GENERATION);
        assert_eq!(furnator.energy, 20);
        assert_eq!(furnator.carbon, 47_980);
    }

    #[test]
    fn one_coal_yields_forty_eight_thousand_fe_over_two_thousand_four_hundred_ticks() {
        let mut furnator = Furnator::new();
        assert!(furnator.load_fuel(1_600));
        let mut ticks = 0;
        while furnator.burning() {
            // A cell next to it keeps taking the energy, so the buffer never fills.
            furnator.energy = 0;
            assert_eq!(furnator.tick(), FURNATOR_GENERATION);
            ticks += 1;
        }
        assert_eq!(ticks, 2_400, "48,000 carbon at 20 FE/t");
        assert_eq!(furnator.carbon, 0);
    }

    #[test]
    fn a_full_buffer_pauses_production_without_burning_carbon() {
        let mut furnator = Furnator {
            energy: FURNATOR_CAPACITY,
            carbon: 100,
        };
        assert_eq!(furnator.tick(), 0);
        assert_eq!(furnator.carbon, 100);
        assert_eq!(furnator.energy, FURNATOR_CAPACITY);

        let mut nearly = Furnator {
            energy: FURNATOR_CAPACITY - 5,
            carbon: 100,
        };
        assert_eq!(nearly.tick(), 5);
        assert_eq!(nearly.energy, FURNATOR_CAPACITY);
        assert_eq!(
            nearly.carbon, 95,
            "only the stored energy comes off the carbon"
        );
    }

    #[test]
    fn fuel_is_only_taken_when_no_carbon_is_left() {
        let mut furnator = Furnator::new();
        assert!(furnator.wants_fuel());
        assert!(!furnator.load_fuel(0), "not a fuel");
        assert!(furnator.load_fuel(200));
        assert!(!furnator.wants_fuel(), "still burning");
        assert!(!furnator.load_fuel(1_600), "one fuel item at a time");

        // Upstream's refuel block does not look at the buffer: fuel becomes
        // carbon and burns once there is room for the energy.
        let mut full = Furnator {
            energy: FURNATOR_CAPACITY,
            carbon: 0,
        };
        assert!(full.wants_fuel(), "a full buffer still takes the next fuel");
        assert!(full.load_fuel(1_600));
        assert_eq!(full.carbon, 48_000);
        assert_eq!(full.tick(), 0, "no room, so nothing is produced");
        assert_eq!(full.carbon, 48_000, "and no carbon is burnt");
    }

    #[test]
    fn a_cable_moves_energy_from_a_source_to_a_sink_up_to_its_rate() {
        let mut sides = empty();
        sides[0] = Some(buffer(10_000, 20_000, 0, 80)); // furnator below: 80 a call
        sides[1] = Some(buffer(0, 1_000_000, 1_000, 1_000)); // cell above
        assert_eq!(
            cable_moves(&sides, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 80
            }],
            "the furnator gives at most its own 80 a call"
        );

        let mut cell = empty();
        cell[0] = Some(buffer(10_000, 1_000_000, 1_000, 1_000)); // a cell gives 1,000
        cell[1] = Some(buffer(0, 1_000_000, 1_000, 1_000));
        assert_eq!(
            cable_moves(&cell, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 500
            }],
            "a cell can give more, so the cable's own 500 caps it"
        );
    }

    #[test]
    fn a_cable_stops_at_the_sink_room_the_source_and_the_per_call_limit() {
        let mut sides = empty();
        sides[0] = Some(buffer(10, 20_000, 0, 80)); // 10 FE only
        sides[1] = Some(buffer(999_990, 1_000_000, 1_000, 1_000)); // 10 room
        assert_eq!(
            cable_moves(&sides, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 10
            }],
            "min(source 10, room 10, transfer 500)"
        );

        let mut limited = empty();
        limited[0] = Some(buffer(10_000, 20_000, 0, 40)); // 40 per call
        limited[1] = Some(buffer(0, 1_000_000, 30, 1_000)); // 30 per call
        assert_eq!(
            cable_moves(&limited, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 30
            }]
        );
    }

    #[test]
    fn a_cable_with_no_source_or_no_sink_moves_nothing() {
        let mut sink_only = empty();
        sink_only[2] = Some(buffer(0, 100, 100, 0));
        assert!(cable_moves(&sink_only, CABLE_TRANSFER, 0).is_empty());

        let mut source_only = empty();
        source_only[2] = Some(buffer(100, 100, 0, 100));
        assert!(cable_moves(&source_only, CABLE_TRANSFER, 0).is_empty());

        assert!(cable_moves(&empty(), CABLE_TRANSFER, 0).is_empty());
    }

    #[test]
    fn a_move_never_starts_and_ends_at_the_same_neighbour() {
        let mut sides = empty();
        sides[0] = Some(buffer(5_000, 10_000, 500, 500)); // a full battery on one side
        let moves = cable_moves(&sides, CABLE_TRANSFER, 0);
        assert!(
            moves.is_empty(),
            "one battery cannot charge itself: {moves:?}"
        );
    }

    #[test]
    fn two_sources_share_the_cable_by_the_rotation_and_the_budget() {
        let mut sides = empty();
        sides[0] = Some(buffer(10_000, 20_000, 0, 500));
        sides[1] = Some(buffer(10_000, 20_000, 0, 500));
        sides[2] = Some(buffer(0, 1_000_000, 1_000, 0));
        let moves = cable_moves(&sides, 600, 0);
        let total: i32 = moves.iter().map(|m| m.amount).sum();
        assert_eq!(total, 600, "the cable rate caps both sources together");
        assert_eq!(moves[0].from, 0, "the first move starts at side 0");
        assert_eq!(moves[0].amount, 500);

        let rotated = cable_moves(&sides, 600, 1);
        assert_eq!(
            rotated[0].from, 1,
            "side 1 goes first when the rotation starts there"
        );
    }

    #[test]
    fn a_network_relays_across_the_cables_between_the_source_and_the_sink() {
        let ports = [
            buffer(1_000, 1_000_000, 1_000, 1_000),
            buffer(0, 1_000_000, 1_000, 1_000),
        ];
        assert_eq!(
            network_moves(&ports, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: CABLE_TRANSFER
            }],
            "a source and a sink on two different cables of one network"
        );
    }

    #[test]
    fn a_network_spends_one_cable_s_rate_not_a_rate_per_cable() {
        let ports = [
            buffer(10_000, 20_000, 0, 500),
            buffer(10_000, 20_000, 0, 500),
            buffer(0, 1_000_000, 1_000, 0),
        ];
        let moves = network_moves(&ports, CABLE_TRANSFER, 0);
        let total: i32 = moves.iter().map(|m| m.amount).sum();
        assert_eq!(total, CABLE_TRANSFER, "the network is one relay");
        assert_eq!(
            moves[0],
            Transfer {
                from: 0,
                to: 2,
                amount: CABLE_TRANSFER
            },
            "the first source spends the whole budget"
        );
    }

    #[test]
    fn one_cable_is_the_same_plan_as_its_six_sides() {
        let mut sides = empty();
        sides[0] = Some(buffer(1_000, 20_000, 0, 80));
        sides[1] = Some(buffer(0, 1_000_000, 1_000, 1_000));
        let ports = [sides[0].unwrap(), sides[1].unwrap()];
        let six = cable_moves(&sides, CABLE_TRANSFER, 0);
        let net = network_moves(&ports, CABLE_TRANSFER, 0);
        assert_eq!(six.len(), 1, "one move, as slice 1's cable made");
        assert_eq!(net.len(), 1);
        assert_eq!(six[0].amount, net[0].amount);
        assert_eq!(six[0].amount, 80, "the source's own per-call limit");

        let mut shared = empty();
        shared[0] = Some(buffer(10_000, 20_000, 0, 500));
        shared[1] = Some(buffer(10_000, 20_000, 0, 500));
        shared[2] = Some(buffer(0, 1_000_000, 1_000, 0));
        let ports = [shared[0].unwrap(), shared[1].unwrap(), shared[2].unwrap()];
        let six = cable_moves(&shared, CABLE_TRANSFER, 1);
        let net = network_moves(&ports, CABLE_TRANSFER, 1);
        assert_eq!(six[0].from, 1);
        assert_eq!(net[0].from, 1, "the rotation moves the same way");
        assert_eq!(six[0].amount, net[0].amount);
    }

    #[test]
    fn a_network_without_a_giver_or_a_taker_moves_nothing() {
        assert!(
            network_moves(&[], CABLE_TRANSFER, 0).is_empty(),
            "no port, no move"
        );
        let sink_only = [buffer(0, 100, 100, 0)];
        assert!(
            network_moves(&sink_only, CABLE_TRANSFER, 0).is_empty(),
            "nothing in the network can give"
        );
        let source_only = [buffer(100, 100, 0, 100)];
        assert!(
            network_moves(&source_only, CABLE_TRANSFER, 0).is_empty(),
            "one port cannot give to itself"
        );
        let battery = [buffer(100, 1_000, 100, 100)];
        assert!(
            network_moves(&battery, CABLE_TRANSFER, 0).is_empty(),
            "a battery cannot charge itself"
        );
    }

    #[test]
    fn every_energizing_rod_tier_scales_upstreams_numbers() {
        // `DefaultEnergies`: energizingTransfer = baseScaling * 100 and
        // energizingCapacity = energizingTransfer * 100.
        let expected = [
            (10_000, 100),
            (40_000, 400),
            (100_000, 1_000),
            (400_000, 4_000),
            (1_000_000, 10_000),
            (4_000_000, 40_000),
            (20_000_000, 200_000),
        ];
        for (index, (capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = rod_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "rod tier {index}'s numbers"
            );
        }
        assert_eq!(
            rod_tier(ROD_TIER_COUNT),
            None,
            "there is no eighth rod tier"
        );
    }

    #[test]
    fn every_energizing_recipe_is_the_jars_own_row() {
        let by_output: Vec<(&str, i32, i32)> = energizing()
            .iter()
            .map(|recipe| (recipe.output, recipe.count, recipe.energy))
            .collect();
        assert_eq!(
            by_output,
            vec![
                ("powah:steel_energized", 2, 10_000),
                ("powah:crystal_blazing", 1, 120_000),
                ("powah:crystal_blazing", 1, 120_000),
                ("powah:crystal_niotic", 1, 300_000),
                ("powah:crystal_spirited", 1, 1_000_000),
                ("powah:crystal_nitro", 16, 20_000_000),
            ],
            "the jar's five materials, with its second blazing variant (one blaze rod)"
        );
        let nitro = energizing()
            .iter()
            .find(|recipe| recipe.output == "powah:crystal_nitro")
            .expect("the nitro recipe");
        assert_eq!(
            nitro.ingredients.len(),
            4,
            "a nether star, two redstone blocks and a blazing crystal block"
        );
    }

    #[test]
    fn an_orb_matches_its_recipe_in_any_slot_order_and_nothing_else() {
        let steel = energize(&["minecraft:iron_ingot", "minecraft:gold_ingot"])
            .expect("iron and gold are the energized steel recipe");
        assert_eq!(steel.output, "powah:steel_energized");
        assert_eq!(
            energize(&["minecraft:gold_ingot", "minecraft:iron_ingot"]),
            Some(steel),
            "the slots are a bag of ingredients, not a pattern"
        );
        assert_eq!(energize(&["minecraft:iron_ingot"]), None, "half a recipe");
        assert_eq!(
            energize(&["minecraft:iron_ingot", "minecraft:iron_ingot"]),
            None,
            "two of one, none of the other"
        );
        assert_eq!(
            energize(&["minecraft:iron_ingot", "minecraft:diamond"]),
            None,
            "one wrong ingredient"
        );
        assert_eq!(energize(&[]), None, "an empty orb has no recipe");
    }

    #[test]
    fn a_stack_of_blaze_powder_in_one_slot_is_one_filled_slot() {
        // Upstream's input slots hold one item each, so four blaze powder are four
        // slots; a single stack is one filled slot and matches one ingredient only.
        assert_eq!(
            energize(&["minecraft:blaze_powder"]),
            None,
            "one powder is a quarter of the blazing recipe"
        );
        let four = [
            "minecraft:blaze_powder",
            "minecraft:blaze_powder",
            "minecraft:blaze_powder",
            "minecraft:blaze_powder",
        ];
        assert_eq!(
            energize(&four).map(|recipe| recipe.output),
            Some("powah:crystal_blazing")
        );
        assert_eq!(
            energize(&["minecraft:blaze_rod"]).map(|recipe| recipe.output),
            Some("powah:crystal_blazing"),
            "the jar's second blazing recipe takes one blaze rod"
        );
    }

    #[test]
    fn the_orb_holds_the_recipes_energy_clears_a_changed_recipe_and_produces_when_full() {
        let steel = energizing()
            .iter()
            .find(|recipe| recipe.output == "powah:steel_energized")
            .expect("the steel recipe");
        let iron_gold = ["minecraft:iron_ingot", "minecraft:gold_ingot"];

        assert_eq!(
            orb_step(&iron_gold, 0, None),
            OrbStep {
                recipe: Some(steel),
                energy: 0,
                done: false
            },
            "a fresh orb holds the recipe and no energy"
        );
        assert_eq!(
            orb_step(&iron_gold, 5_000, Some("powah:steel_energized")).energy,
            5_000,
            "a rod's fill stays in a buffer under the recipe's 10,000"
        );
        assert_eq!(
            orb_step(&iron_gold, 5_000, None).energy,
            5_000,
            "energy a rod moved in before the orb recorded a recipe is adopted, not thrown away"
        );
        assert_eq!(
            orb_step(&iron_gold, 10_000, Some("powah:steel_energized")),
            OrbStep {
                recipe: Some(steel),
                energy: 0,
                done: true
            },
            "at the recipe's energy the orb produces and empties"
        );
        assert_eq!(
            orb_step(&iron_gold, 5_000, Some("powah:crystal_niotic")).energy,
            0,
            "a different recipe clears what the last one stored (onSlotChanged)"
        );
        assert!(
            orb_step(
                &["minecraft:diamond"],
                400_000,
                Some("powah:crystal_niotic")
            )
            .done,
            "a buffer above the recipe's energy is done, not clamped back"
        );
        assert_eq!(
            orb_step(&[], 5_000, None),
            OrbStep {
                recipe: None,
                energy: 0,
                done: false
            },
            "an orb with no recipe stores nothing"
        );
        assert_eq!(
            orb_step(&["minecraft:diamond"], 0, None)
                .recipe
                .map(|recipe| recipe.output),
            Some("powah:crystal_niotic"),
            "one diamond is the niotic recipe"
        );
    }

    #[test]
    fn an_orb_takes_the_room_it_has_and_the_rod_gives_no_more_than_it_holds() {
        assert_eq!(
            orb_fill(200_000, 200_000, 0, 120_000),
            120_000,
            "a nitro rod offers 200,000 into 120,000 of room: the orb takes 120,000"
        );
        assert_eq!(
            orb_fill(10_000, 100, 0, 10_000),
            100,
            "a starter rod gives its own 100 a tick"
        );
        assert_eq!(
            orb_fill(50_000, 200_000, 119_950, 120_000),
            50,
            "the last 50 FE of the recipe"
        );
        assert_eq!(
            orb_fill(0, 200_000, 0, 120_000),
            0,
            "an empty rod gives nothing"
        );
        assert_eq!(
            orb_fill(10_000, 100, 10_000, 10_000),
            0,
            "a full orb has no room"
        );
        assert_eq!(orb_fill(-5, 100, 0, 10_000), 0);
    }

    #[test]
    fn a_rod_warms_up_for_twenty_ticks_and_then_fills_every_tick() {
        let mut cooldown = 0;
        for tick in 1..=ROD_WARMUP {
            cooldown = rod_cooldown(true, cooldown);
            assert_eq!(cooldown, tick, "tick {tick} of the warm-up");
        }
        assert_eq!(
            rod_cooldown(true, cooldown),
            ROD_WARMUP,
            "the warm-up holds at 20: upstream's Ticker.ended() stays true, so a warm rod fills every tick"
        );
        assert_eq!(
            rod_cooldown(false, ROD_WARMUP),
            ROD_WARMUP - 1,
            "an idle rod cools"
        );
        assert_eq!(rod_cooldown(false, 0), 0, "and stops at zero");
    }

    #[test]
    fn the_network_rotation_starts_at_another_port() {
        let ports = [
            buffer(10_000, 20_000, 0, 500),
            buffer(10_000, 20_000, 0, 500),
            buffer(0, 1_000_000, 1_000, 0),
        ];
        assert_eq!(network_moves(&ports, CABLE_TRANSFER, 0)[0].from, 0);
        assert_eq!(network_moves(&ports, CABLE_TRANSFER, 1)[0].from, 1);
        assert_eq!(
            network_moves(&ports, CABLE_TRANSFER, 11)[0].from,
            0,
            "a rotation past the port count wraps: 11 % 3 leaves the sink first"
        );
    }

    #[test]
    fn a_cable_spreads_one_source_over_several_sinks_without_overdrawing() {
        let mut sides = empty();
        sides[0] = Some(buffer(1_000, 20_000, 0, 500));
        sides[1] = Some(buffer(0, 100, 100, 0)); // 100 room
        sides[2] = Some(buffer(0, 100, 100, 0)); // 100 room
        sides[3] = Some(buffer(0, 1_000_000, 1_000, 0));
        let moves = cable_moves(&sides, CABLE_TRANSFER, 0);
        let moved: i32 = moves.iter().map(|m| m.amount).sum();
        assert_eq!(moved, 500, "the cable carries 500 in total");
        assert_eq!(
            moves[0],
            Transfer {
                from: 0,
                to: 1,
                amount: 100
            }
        );
        assert_eq!(
            moves[1],
            Transfer {
                from: 0,
                to: 2,
                amount: 100
            }
        );
        assert_eq!(
            moves[2],
            Transfer {
                from: 0,
                to: 3,
                amount: 300
            }
        );
    }
}
