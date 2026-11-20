//! The Powah machines' glue: the three block entity Parts, the level-tick queue,
//! the block energy buffers and the `/powah` command bodies.
//!
//! Every rule lives in [`crate::runtime`] (the retired bundle's `runtime.rs`,
//! byte-identical); this file reads the game, calls it and writes the result
//! back. Its tests pin what it *declares* (one fuel slot, two buffers and the
//! cable's none), that the saved compound reads back, and the exact lines
//! `/powah info` prints.
//!
//! What the retired bundle's `block_entity_energy` hook did through a shared
//! output is a declaration now (`Registrar::block_energy`, keyed by the block),
//! so the buffer's numbers have one home, [`battery_of`], which the cable's
//! relay also reads when it plans a move.
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use mod_api::{
    self, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply, BlockEntityTicksEvent,
    BlockEntityTypeKey, BlockPart, Command, CommandArgumentValue, Compound, Direction, ENERGY,
    Host, HostTransaction, LevelRef, ModContainer, ModEnergyBuffer, Registrar, ServerHost, Tag,
    move_energy,
};

use crate::runtime::{
    CABLE_TRANSFER, CELL_CAPACITY, CELL_TRANSFER, FURNATOR_CAPACITY, FURNATOR_TRANSFER, Furnator,
    Neighbour, cable_moves, clamp_energy, comparator_strength, push_amount,
};

/// The furnator block and its item.
pub(crate) const FURNATOR: &str = "powah:furnator_starter";
/// The energy cell block and its item.
pub(crate) const CELL: &str = "powah:energy_cell_starter";
/// The energy cable block and its item.
pub(crate) const CABLE: &str = "powah:energy_cable_starter";

/// The block entity type paths, in this mod's namespace.
pub(crate) const FURNATOR_TYPE: &str = "furnator_starter";
pub(crate) const CELL_TYPE: &str = "energy_cell_starter";
pub(crate) const CABLE_TYPE: &str = "energy_cable_starter";

/// The furnator's fuel slot: upstream's slot 1 of 2 (the charging slot is a gap),
/// so slice 1 keeps exactly one.
const FUEL_SLOT: usize = 0;
const SLOT_COUNT: usize = 1;

/// Upstream's saved keys (`saveAdditional`).
const KEY_ENERGY: &str = "energy";
const KEY_CARBON: &str = "carbon";

/// The three machines, as a readout has to tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Furnator,
    Cell,
    Cable,
}

impl Kind {
    /// The machine a block id names, `None` for anything else.
    pub(crate) fn of(block: &str) -> Option<Kind> {
        match block {
            FURNATOR => Some(Kind::Furnator),
            CELL => Some(Kind::Cell),
            CABLE => Some(Kind::Cable),
            _ => None,
        }
    }
}

/// What `/powah set` did, so the command can answer honestly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetOutcome {
    /// The buffer now holds this much.
    Set(i32),
    /// No Powah machine stands there.
    NoBlock,
    /// The cable stores nothing, so there is nothing to set.
    Cable,
}

/// One block entity type, built in `Hooks::register`: the type's id is this mod's
/// namespace plus `name`, and `name` is the one block allowed to hold it.
pub(crate) fn declare(registrar: &mut dyn Registrar, name: &str) -> BlockEntityTypeKey {
    registrar.block_entity_type(name).blocks(&[name]).build()
}

/// The furnator's container: one fuel slot every face reaches, no menu (slice 1
/// has no GUI). Upstream has two slots — the charging slot is a gap.
pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: SLOT_COUNT,
        faces: std::array::from_fn(|_| vec![FUEL_SLOT]),
        inputs: vec![FUEL_SLOT],
        outputs: vec![FUEL_SLOT],
        menu: None,
    }
}

/// The furnator's buffer (`AbstractEnergyProvider`): 20,000 FE, filled by its own
/// burn only, given out at 80 a side.
pub(crate) fn furnator_battery() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: FURNATOR_CAPACITY,
        max_insert: 0,
        max_extract: FURNATOR_TRANSFER,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    }
}

/// The cell's buffer (`AbstractEnergyStorage`): 1,000,000 FE, 1,000 a side both ways.
pub(crate) fn cell_battery() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: CELL_CAPACITY,
        max_insert: CELL_TRANSFER,
        max_extract: CELL_TRANSFER,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// The buffer a block declares; the cable stores nothing (upstream's capacity is
/// 0), so it declares none and is only ever a relay.
pub(crate) fn battery_of(block: &str) -> Option<ModEnergyBuffer> {
    match Kind::of(block) {
        Some(Kind::Furnator) => Some(furnator_battery()),
        Some(Kind::Cell) => Some(cell_battery()),
        _ => None,
    }
}

/// The furnator saved in a block entity's compound; a fresh one is empty.
pub(crate) fn read(saved: &Tag) -> Furnator {
    Furnator {
        energy: clamp_energy(int(saved, KEY_ENERGY), FURNATOR_CAPACITY),
        carbon: int(saved, KEY_CARBON).max(0),
    }
}

/// Write the tick's result back under upstream's keys: the buffer's amount
/// (`energy`, the int the energy capability reads) and the carbon left.
pub(crate) fn write(saved: &mut Tag, state: &Furnator) {
    let Tag::Compound(nbt) = saved else {
        return;
    };
    nbt.insert(KEY_ENERGY.into(), Tag::Int(state.energy));
    nbt.insert(KEY_CARBON.into(), Tag::Int(state.carbon));
}

/// A saved int, 0 when absent or of another type.
fn int(saved: &Tag, key: &str) -> i32 {
    match saved.get(key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

/// One machine's readout line, exactly as 0.1.0 printed it (the line the
/// in-game check's assertions match, so it must not drift).
pub(crate) fn status_line(
    kind: Kind,
    pos: [i32; 3],
    energy: i32,
    carbon: i32,
    fuel: Option<(&str, u32)>,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    match kind {
        Kind::Furnator => {
            let fuel = match fuel {
                Some((item, count)) => format!("{item} x{count}"),
                None => "empty".to_owned(),
            };
            format!(
                "Furnator at {x},{y},{z}: {energy}/{FURNATOR_CAPACITY} FE (comparator {}, \
                 carbon {carbon}, burning {}, fuel {fuel}, t={time})",
                comparator_strength(energy, FURNATOR_CAPACITY),
                carbon > 0,
            )
        }
        Kind::Cell => format!(
            "Energy Cell at {x},{y},{z}: {energy}/{CELL_CAPACITY} FE (comparator {}, t={time})",
            comparator_strength(energy, CELL_CAPACITY),
        ),
        Kind::Cable => format!(
            "Energy Cable at {x},{y},{z}: no storage, moves up to {CABLE_TRANSFER} FE/t (t={time})"
        ),
    }
}

/// The machines whose block entity ticked this tick, per level, drained at the
/// end of that level's tick. The SDK cannot enumerate a level's block entities
/// and a ticking one is out of the world, so the tick leaves its position here
/// and the moves run where every machine is back in the world.
#[derive(Default)]
pub(crate) struct Queue {
    pending: HashMap<LevelRef, BTreeSet<[i32; 3]>>,
}

impl Queue {
    /// Queue `pos` for the level the machine's tick ran in.
    pub(crate) fn push(&mut self, level: LevelRef, pos: [i32; 3]) {
        self.pending.entry(level).or_default().insert(pos);
    }

    /// Everything queued for `level`, in position order (empty afterwards).
    pub(crate) fn take(&mut self, level: LevelRef) -> BTreeSet<[i32; 3]> {
        self.pending.remove(&level).unwrap_or_default()
    }

    /// Forget every queued position: a world opened or closed.
    pub(crate) fn clear(&mut self) {
        self.pending.clear();
    }
}

/// The level the server ticks now: the one a block-entity tick runs in.
pub(crate) fn level_now(host: &dyn ServerHost) -> Option<LevelRef> {
    host.level(host.dimension())
}

/// The id of the block at `pos`, if there is one.
pub(crate) fn block_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    Some(level.block(host, pos)?.id(host))
}

/// The two usages `/powah` declares, both operator-only (level 2). They are
/// administrative: `set` writes a machine's stored FE and `info` reads its fuel
/// slot, at coordinates the caller chooses, so they are operator diagnostics
/// rather than gameplay (review finding 2026-11-18).
pub(crate) fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::op("powah info <pos:block_pos>", 2));
    commands.push(Command::op("powah set <amount:integer> <pos:block_pos>", 2));
}

/// A machine's readout, or `None` when no Powah machine stands there. The block id
/// is the test, so a chest at that position is "no Powah block", not a machine at
/// 0 FE.
pub(crate) fn info_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<String> {
    let level = level_now(host)?;
    let kind = Kind::of(&block_at(&*host, level, pos)?)?;
    let time = level.game_time(&*host).unwrap_or(0);
    let (energy, carbon, fuel) = match kind {
        Kind::Furnator => {
            let entity = host.block_entity(level, pos)?;
            let state = read(&Tag::Compound(entity.data(&*host)?));
            let fuel = entity
                .item_slot(&*host, FUEL_SLOT as u32)
                .ok()
                .flatten()
                .map(|stack| (stack.item().id(&*host), stack.count()));
            (state.energy, state.carbon, fuel)
        }
        Kind::Cell => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            (
                clamp_energy(int(&saved, KEY_ENERGY), CELL_CAPACITY),
                0,
                None,
            )
        }
        Kind::Cable => (0, 0, None),
    };
    let fuel = fuel.as_ref().map(|(item, count)| (item.as_str(), *count));
    Some(status_line(kind, pos, energy, carbon, fuel, time))
}

/// Set a furnator's or a cell's stored FE (`/powah set`): the harness control the
/// in-game check uses. Writing an `energy` key into a foreign block's compound
/// would be a foreign key in someone else's data, so only this mod's blocks take it.
pub(crate) fn set_energy(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> SetOutcome {
    let Some(level) = level_now(host) else {
        return SetOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return SetOutcome::NoBlock;
    };
    let capacity = match Kind::of(&block) {
        Some(Kind::Furnator) => FURNATOR_CAPACITY,
        Some(Kind::Cell) => CELL_CAPACITY,
        Some(Kind::Cable) => return SetOutcome::Cable,
        None => return SetOutcome::NoBlock,
    };
    let energy = clamp_energy(amount, capacity);
    let Some(entity) = host.block_entity(level, pos) else {
        return SetOutcome::NoBlock;
    };
    let mut patch = Compound::new();
    patch.insert(KEY_ENERGY.into(), Tag::Int(energy));
    // `merge_data` is `/data merge block`: it loads the patch and marks the chunk
    // changed, which is the `setChanged` the retired bundle had to fake.
    match entity.merge_data(host, patch) {
        Ok(_) => SetOutcome::Set(energy),
        Err(_) => SetOutcome::NoBlock,
    }
}

/// The furnator: a block entity that burns its fuel and ticks.
pub(crate) struct FurnatorPart {
    queue: Arc<Mutex<Queue>>,
}

impl FurnatorPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>) -> FurnatorPart {
        FurnatorPart { queue }
    }
}

#[mod_api::horizon_part]
impl BlockPart for FurnatorPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for FurnatorPart {
    /// `EntityBlock.getTicker`: it ticks. Never clear a value an earlier mod set.
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: load the fuel upstream would, burn one tick of carbon,
    /// then queue this position for the level's move phase. The payload is left
    /// `Changed` only when something moved, so an idle furnator does not dirty its
    /// chunk every tick.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let changed = burn(host, event);
        self.enqueue(host, event.pos);
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

impl FurnatorPart {
    /// Queue `pos` for the level this tick runs in; outside a ticking level there
    /// is nothing to move and nothing is queued.
    fn enqueue(&self, host: &dyn ServerHost, pos: [i32; 3]) {
        if let Some(level) = level_now(host) {
            // Only this expression runs under the lock: one insert on the port's own
            // state, which cannot panic and so cannot poison it.
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, pos);
        }
    }
}

/// The cell and the cable: a block entity that stores nothing of its own and only
/// queues itself for the level tick's end, where the moves run.
pub(crate) struct IdlePart {
    queue: Arc<Mutex<Queue>>,
}

impl IdlePart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>) -> IdlePart {
        IdlePart { queue }
    }
}

#[mod_api::horizon_part]
impl BlockPart for IdlePart {}

#[mod_api::horizon_part]
impl BlockEntityPart for IdlePart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// The cell stores its FE in its saved compound and the cable in nothing, so
    /// neither has a rule of its own: both only queue the move phase.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if let Some(level) = level_now(host) {
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, event.pos);
        }
        *event.reply = BlockEntityTickReply::Unchanged;
    }
}

/// The furnator's tick: `initBurn` (`AbstractGeneratorBlock`), one tick of
/// production, and the state written back. Answers whether anything changed.
fn burn(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) -> bool {
    let resized = event.entity.items.len() != SLOT_COUNT;
    if resized {
        event.entity.items.resize(SLOT_COUNT, None);
    }
    let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
    let before = read(&saved);
    let mut state = before;
    let mut took = false;
    if state.wants_fuel() {
        let mut fuel = FuelSlot {
            host: &*host,
            entity: &mut event.entity,
        };
        let burn_time = fuel.peek();
        if state.load_fuel(burn_time) {
            fuel.take();
            took = true;
        }
    }
    let produced = state.tick() > 0;
    write(&mut saved, &state);
    if let Tag::Compound(nbt) = saved {
        event.entity.nbt = nbt;
    }
    resized || took || produced
}

/// The fuel slot, as `AbstractGeneratorBlock.initBurn` reads it: the item's
/// smelting burn time and the shrink (or the crafting remainder) that takes it.
struct FuelSlot<'a, 'g> {
    host: &'a dyn ServerHost,
    entity: &'a mut mod_api::ModBlockEntityView<'g>,
}

impl FuelSlot<'_, '_> {
    /// The burn time the item in the fuel slot gives, 0 when there is none.
    fn peek(&self) -> u32 {
        let Some(Some(stack)) = self.entity.items.first() else {
            return 0;
        };
        burn_time(self.host, stack)
    }

    /// `ItemStack.shrink(1)` plus `getCraftingRemainingItem`: a lava bucket leaves
    /// its bucket.
    fn take(&mut self) {
        let Some(Some(stack)) = self.entity.items.first() else {
            return;
        };
        let left = stack.clone().shrink(1);
        self.entity.items[FUEL_SLOT] = left.or_else(|| remaining(self.host, stack));
    }
}

/// The engine's fuel value (`minecraft:cooking_fuel.burn_time`), 0 if none.
fn burn_time(host: &dyn Host, stack: &mod_api::ItemStack) -> u32 {
    host.item_burn_time(stack.item()).unwrap_or(0).max(0) as u32
}

/// `ItemStack.getCraftingRemainingItem`: the bucket a lava bucket leaves.
fn remaining(host: &dyn Host, stack: &mod_api::ItemStack) -> Option<mod_api::ItemStack> {
    host.item_crafting_remainder(stack)
}

/// After the level's tick: run the queued moves. A block entity is out of the
/// world during its own tick, so every machine reads its neighbours here, where
/// they are all back in the world.
pub(crate) fn on_level_tick_post(
    host: &mut dyn ServerHost,
    level: LevelRef,
    queued: BTreeSet<[i32; 3]>,
) {
    for pos in queued {
        let Some(kind) = block_at(&*host, level, pos).as_deref().and_then(Kind::of) else {
            continue;
        };
        match kind {
            Kind::Furnator => push_out(host, level, pos, FURNATOR_TRANSFER),
            Kind::Cell => push_out(host, level, pos, CELL_TRANSFER),
            Kind::Cable => relay(host, level, pos),
        }
    }
}

/// Upstream's `extractFromSides`: at most the transfer and what is stored, into
/// every side whose neighbour can take it. One transaction per side, so a refused
/// insert creates no energy.
fn push_out(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], transfer: i32) {
    let Some(own) = level.capability(&mut *host, &ENERGY, pos, Some(Direction::Up)) else {
        return;
    };
    let stored = i32::try_from(own.amount(host)).unwrap_or(i32::MAX);
    let amount = push_amount(stored, transfer);
    if amount == 0 {
        return;
    }
    for side in Direction::ALL {
        let neighbour = offset(pos, side);
        // Upstream's `extractFromSides` skips a side whose neighbour refuses a fill; this
        // mod's own blocks say so in their declaration (`furnator_battery` has no insert
        // face), and a foreign block is tried, its handler refusing.
        let Some(target_block) = block_at(&*host, level, neighbour) else {
            continue;
        };
        if battery_of(&target_block)
            .is_some_and(|buffer| !buffer.insert_faces[side.opposite() as usize])
        {
            continue;
        }
        let Some(target) = level.capability(&mut *host, &ENERGY, neighbour, Some(side.opposite()))
        else {
            continue;
        };
        let room = target.capacity(host).saturating_sub(target.amount(host));
        let want = u32::try_from(amount).unwrap_or(0).min(room);
        if want == 0 {
            continue;
        }
        let _ = host.transaction(|host, tx| -> Result<u32, ()> {
            let Some(own) = level.capability(&mut *host, &ENERGY, pos, Some(side)) else {
                return Ok(0);
            };
            let Some(target) =
                level.capability(&mut *host, &ENERGY, neighbour, Some(side.opposite()))
            else {
                return Ok(0);
            };
            move_energy(host, &own, &target, want, tx).map_err(|_| ())
        });
    }
}

/// The cable's move: it reads each neighbour through the side facing it and lets
/// the pure planner decide where the energy goes (upstream's round-robin is the
/// rotation, seeded by the game time). The cable stores nothing, so no move starts
/// and ends at the same neighbour.
fn relay(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    let mut sides = [None; 6];
    for (index, side) in Direction::ALL.into_iter().enumerate() {
        sides[index] = neighbour(host, level, offset(pos, side), side.opposite());
    }
    let rotation = level.game_time(&*host).unwrap_or(0).rem_euclid(6) as usize;
    for step in cable_moves(&sides, CABLE_TRANSFER, rotation) {
        let from = Direction::ALL[step.from];
        let to = Direction::ALL[step.to];
        let source_pos = offset(pos, from);
        let target_pos = offset(pos, to);
        let want = u32::try_from(step.amount).unwrap_or(0);
        let _ = host.transaction(|host, tx| -> Result<u32, ()> {
            let Some(source) =
                level.capability(&mut *host, &ENERGY, source_pos, Some(from.opposite()))
            else {
                return Ok(0);
            };
            let Some(target) =
                level.capability(&mut *host, &ENERGY, target_pos, Some(to.opposite()))
            else {
                return Ok(0);
            };
            move_energy(host, &source, &target, want, tx).map_err(|_| ())
        });
    }
}

/// A neighbour's buffer as the cable reaches it. The capability answers `amount`,
/// `capacity` and the moves themselves; the per-call limits and the faces are this
/// mod's own declaration, read back through [`battery_of`], so its own three blocks
/// are planned with the numbers they really enforce. A foreign block's limits are
/// not visible through the capability, so its handler caps the move instead.
fn neighbour(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    side: Direction,
) -> Option<Neighbour> {
    let block = block_at(&*host, level, pos)?;
    let handler = level.capability(&mut *host, &ENERGY, pos, Some(side))?;
    let energy = i32::try_from(handler.amount(host)).unwrap_or(i32::MAX);
    let capacity = i32::try_from(handler.capacity(host)).unwrap_or(i32::MAX);
    let buffer = battery_of(&block);
    let face = side as usize;
    Some(Neighbour {
        energy,
        capacity,
        max_insert: buffer.as_ref().map_or(i32::MAX, |b| b.max_insert),
        max_extract: buffer.as_ref().map_or(i32::MAX, |b| b.max_extract),
        can_insert: buffer.as_ref().is_none_or(|b| b.insert_faces[face]),
        can_extract: buffer.as_ref().is_none_or(|b| b.extract_faces[face]),
    })
}

/// The neighbouring position one side leads to.
fn offset(pos: [i32; 3], side: Direction) -> [i32; 3] {
    let [dx, dy, dz] = side.normal();
    [pos[0] + dx, pos[1] + dy, pos[2] + dz]
}

/// `/powah`'s block position argument, as the dispatcher resolved it.
pub(crate) fn block_pos(argument: Option<&CommandArgumentValue>) -> Option<[i32; 3]> {
    match argument? {
        CommandArgumentValue::BlockPos(pos) => Some(*pos),
        _ => None,
    }
}

/// `/powah`'s amount argument.
pub(crate) fn integer(argument: Option<&CommandArgumentValue>) -> Option<i32> {
    match argument? {
        CommandArgumentValue::Integer(amount) => Some(*amount),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The review finding (2026-11-18): both usages act on any loaded machine at
    /// player-chosen coordinates, so a level-0 registration lets a non-operator
    /// create or drain FE and read another player's fuel. Operator-only is the
    /// fix; this test fails on a `Command::new` registration (level 0).
    #[test]
    fn the_harness_commands_are_operator_only() {
        let mut commands = Vec::new();
        declare_commands(&mut commands);
        assert_eq!(commands.len(), 2, "every usage is declared");
        for command in &commands {
            assert!(
                command.usage.starts_with("powah "),
                "{} is this mod's",
                command.usage
            );
            assert_eq!(
                command.level, 2,
                "{} is an administrative harness control",
                command.usage
            );
        }
    }

    #[test]
    fn a_block_id_maps_to_its_machine_and_nothing_else_does() {
        assert_eq!(Kind::of(FURNATOR), Some(Kind::Furnator));
        assert_eq!(Kind::of(CELL), Some(Kind::Cell));
        assert_eq!(Kind::of(CABLE), Some(Kind::Cable));
        assert_eq!(Kind::of("minecraft:chest"), None);
        assert_eq!(Kind::of("powah:furnator_basic"), None, "a later tier");
    }

    #[test]
    fn the_furnator_declares_one_fuel_slot_every_face_reaches() {
        let container = container();
        assert_eq!(container.slots, SLOT_COUNT, "the fuel slot only");
        assert_eq!(container.inputs, vec![FUEL_SLOT]);
        assert_eq!(container.outputs, vec![FUEL_SLOT]);
        assert_eq!(
            container.faces,
            std::array::from_fn(|_| vec![FUEL_SLOT]),
            "automation reaches the fuel slot from every face"
        );
        assert_eq!(container.menu, None, "slice 1 has no GUI");
    }

    #[test]
    fn the_furnator_declares_an_extract_only_buffer() {
        let buffer = furnator_battery();
        assert_eq!(buffer.capacity, FURNATOR_CAPACITY);
        assert_eq!(buffer.max_insert, 0, "it makes its own FE");
        assert_eq!(buffer.max_extract, FURNATOR_TRANSFER);
        assert_eq!(buffer.insert_faces, [false; 6]);
        assert_eq!(buffer.extract_faces, [true; 6]);
    }

    #[test]
    fn the_cell_declares_a_buffer_both_ways() {
        let buffer = cell_battery();
        assert_eq!(buffer.capacity, CELL_CAPACITY);
        assert_eq!(buffer.max_insert, CELL_TRANSFER);
        assert_eq!(buffer.max_extract, CELL_TRANSFER);
        assert_eq!(buffer.insert_faces, [true; 6]);
        assert_eq!(buffer.extract_faces, [true; 6]);
        assert_eq!(battery_of(CELL), Some(buffer));
    }

    #[test]
    fn the_cable_declares_no_buffer_because_it_stores_nothing() {
        assert_eq!(battery_of(CABLE), None);
        assert_eq!(battery_of("minecraft:chest"), None);
    }

    #[test]
    fn a_fresh_compound_is_an_empty_furnator() {
        assert_eq!(read(&Tag::Compound(Compound::new())), Furnator::new());
    }

    #[test]
    fn the_tick_s_state_reads_back_from_the_compound_it_wrote() {
        let mut saved = Tag::Compound(Compound::new());
        let state = Furnator {
            energy: 20_000,
            carbon: 46_800,
        };
        write(&mut saved, &state);
        assert_eq!(read(&saved), state);
        assert_eq!(
            saved.get(KEY_ENERGY),
            Some(&Tag::Int(20_000)),
            "the buffer is the int `energy` the energy capability reads"
        );
        assert_eq!(saved.get(KEY_CARBON), Some(&Tag::Int(46_800)));
    }

    #[test]
    fn the_info_lines_are_the_ones_the_e2e_asserts() {
        assert_eq!(
            status_line(Kind::Furnator, [0, 100, 0], 0, 0, None, 150),
            "Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, \
             t=150)"
        );
        assert_eq!(
            status_line(
                Kind::Furnator,
                [0, 100, 0],
                320,
                46_800,
                Some(("minecraft:coal", 4)),
                282
            ),
            "Furnator at 0,100,0: 320/20000 FE (comparator 0, carbon 46800, burning true, fuel \
             minecraft:coal x4, t=282)"
        );
        assert_eq!(
            status_line(Kind::Cell, [2, 100, 0], 1_360, 0, None, 290),
            "Energy Cell at 2,100,0: 1360/1000000 FE (comparator 0, t=290)"
        );
        assert_eq!(
            status_line(Kind::Cell, [2, 100, 0], 1_000_000, 0, None, 380),
            "Energy Cell at 2,100,0: 1000000/1000000 FE (comparator 15, t=380)"
        );
        assert_eq!(
            status_line(Kind::Cable, [1, 100, 0], 0, 0, None, 188),
            "Energy Cable at 1,100,0: no storage, moves up to 500 FE/t (t=188)"
        );
    }
}
