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

/// The moves one cable makes in a tick: from each neighbour that can give into
/// each that can take, up to the cable's transfer rate in total. `rotation`
/// rotates the starting side so two equal sources share the cable as upstream's
/// round-robin does. A cable stores nothing, so no move starts and ends at the
/// same neighbour, and the total never exceeds `transfer`.
#[must_use]
pub fn cable_moves(
    neighbours: &[Option<Neighbour>; 6],
    transfer: i32,
    rotation: usize,
) -> Vec<Transfer> {
    let mut moves = Vec::new();
    let mut budget = transfer.max(0);
    let mut sink = *neighbours;
    for step in 0..6 {
        if budget == 0 {
            break;
        }
        let from = (rotation + step) % 6;
        let Some(source) = neighbours[from] else {
            continue;
        };
        let mut available = source.available();
        for other in 0..6 {
            if available == 0 || budget == 0 {
                break;
            }
            let to = (rotation + other) % 6;
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
