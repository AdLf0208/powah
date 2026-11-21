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
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use mod_api::{
    self, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply, BlockEntityTicksEvent,
    BlockEntityTypeKey, BlockPart, Command, CommandArgumentValue, Compound, Direction, ENERGY,
    Host, HostTransaction, Identifier, LevelRef, ModContainer, ModEnergyBuffer, Registrar,
    ServerHost, Tag, move_energy,
};

use crate::runtime::{
    CABLE_TIER_COUNT, CELL_TIER_COUNT, CableTier, CellTier, EnergizingRecipe, FURNATOR_TIER_COUNT,
    Furnator, FurnatorTier, Neighbour, ORB_MAX_ENERGY, ORB_OUTPUT_SLOT, ORB_SLOTS, ROD_TIER_COUNT,
    RodTier, cable_tier, cell_tier, clamp_energy, comparator_strength, furnator_tier,
    network_moves, push_amount, rod_tier,
};

/// The energizing orb block and its item.
pub(crate) const ORB: &str = "powah:energizing_orb";
/// The orb's block entity type path, in this mod's namespace.
pub(crate) const ORB_TYPE: &str = "energizing_orb";

/// Every energizing rod tier's block id and block entity type name, in the
/// ladder's order (the index [`rod_tier`] takes).
pub(crate) const RODS: [(&str, &str); ROD_TIER_COUNT] = [
    ("powah:energizing_rod_starter", "energizing_rod_starter"),
    ("powah:energizing_rod_basic", "energizing_rod_basic"),
    ("powah:energizing_rod_hardened", "energizing_rod_hardened"),
    ("powah:energizing_rod_blazing", "energizing_rod_blazing"),
    ("powah:energizing_rod_niotic", "energizing_rod_niotic"),
    ("powah:energizing_rod_spirited", "energizing_rod_spirited"),
    ("powah:energizing_rod_nitro", "energizing_rod_nitro"),
];

/// The starter furnator block and its item (the ladder's first row).
pub(crate) const FURNATOR: &str = "powah:furnator_starter";
/// The starter energy cell block and its item.
pub(crate) const CELL: &str = "powah:energy_cell_starter";
/// The starter energy cable block and its item.
pub(crate) const CABLE: &str = "powah:energy_cable_starter";

/// The block entity type paths, in this mod's namespace.
pub(crate) const FURNATOR_TYPE: &str = "furnator_starter";
pub(crate) const CELL_TYPE: &str = "energy_cell_starter";
pub(crate) const CABLE_TYPE: &str = "energy_cable_starter";

/// Every furnator tier's block id and block entity type name, in the ladder's
/// order (the index [`furnator_tier`] takes).
pub(crate) const FURNATORS: [(&str, &str); FURNATOR_TIER_COUNT] = [
    (FURNATOR, FURNATOR_TYPE),
    ("powah:furnator_basic", "furnator_basic"),
    ("powah:furnator_hardened", "furnator_hardened"),
    ("powah:furnator_blazing", "furnator_blazing"),
    ("powah:furnator_niotic", "furnator_niotic"),
    ("powah:furnator_spirited", "furnator_spirited"),
    ("powah:furnator_nitro", "furnator_nitro"),
];

/// Every energy cable tier's block id and block entity type name, in the
/// ladder's order (the index [`cable_tier`] takes).
pub(crate) const CABLES: [(&str, &str); CABLE_TIER_COUNT] = [
    (CABLE, CABLE_TYPE),
    ("powah:energy_cable_basic", "energy_cable_basic"),
    ("powah:energy_cable_hardened", "energy_cable_hardened"),
    ("powah:energy_cable_blazing", "energy_cable_blazing"),
    ("powah:energy_cable_niotic", "energy_cable_niotic"),
    ("powah:energy_cable_spirited", "energy_cable_spirited"),
    ("powah:energy_cable_nitro", "energy_cable_nitro"),
];

/// Every energy cell tier's block id and block entity type name, in the ladder's
/// order (the index [`cell_tier`] takes). A type name is its block id without this
/// mod's namespace, which is what [`declare`] attaches.
pub(crate) const CELLS: [(&str, &str); CELL_TIER_COUNT] = [
    (CELL, CELL_TYPE),
    ("powah:energy_cell_basic", "energy_cell_basic"),
    ("powah:energy_cell_hardened", "energy_cell_hardened"),
    ("powah:energy_cell_blazing", "energy_cell_blazing"),
    ("powah:energy_cell_niotic", "energy_cell_niotic"),
    ("powah:energy_cell_spirited", "energy_cell_spirited"),
    ("powah:energy_cell_nitro", "energy_cell_nitro"),
];

/// The furnator's fuel slot: upstream's slot 1 of 2 (the charging slot is a gap),
/// so slice 1 keeps exactly one.
const FUEL_SLOT: usize = 0;
const SLOT_COUNT: usize = 1;

/// Upstream's saved keys (`saveAdditional`).
const KEY_ENERGY: &str = "energy";
const KEY_CARBON: &str = "carbon";
/// The orb's matched recipe (its output id) and the rod's linked orb position.
const KEY_RECIPE: &str = "recipe";
const KEY_ORB: &str = "orb";
/// A rod's warm-up counter (`Ticker(20)`), kept across a reload so a reloaded rod
/// does not restart its 20 ticks.
const KEY_COOLDOWN: &str = "cool_down";

/// The three machines, as a readout has to tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A furnator, carrying its tier's generation, capacity and transfer.
    Furnator(FurnatorTier),
    /// An energy cell, carrying its tier's capacity and transfer.
    Cell(CellTier),
    /// An energy cable, carrying its tier's rate.
    Cable(CableTier),
    /// The energizing orb: a recipe-gated buffer over seven slots.
    Orb,
    /// An energizing rod, carrying its tier's capacity and transfer.
    Rod(RodTier),
}

impl Kind {
    /// The machine a block id names, `None` for anything else.
    pub(crate) fn of(block: &str) -> Option<Kind> {
        match block {
            ORB => Some(Kind::Orb),
            _ => furnator_of(block)
                .map(Kind::Furnator)
                .or_else(|| cell_of(block).map(Kind::Cell))
                .or_else(|| cable_of(block).map(Kind::Cable))
                .or_else(|| rod_of(block).map(Kind::Rod)),
        }
    }
}

/// The furnator tier a block id names, with its numbers; `None` for any other
/// block. The ids are the content's own (`data/powah/horizon/block/furnator_*`).
pub(crate) fn furnator_of(block: &str) -> Option<FurnatorTier> {
    let index = FURNATORS.iter().position(|(id, _)| *id == block)?;
    furnator_tier(index)
}

/// The energy cable tier a block id names, with its rate; `None` for any other
/// block. The ids are the content's own (`data/powah/horizon/block/energy_cable_*`).
pub(crate) fn cable_of(block: &str) -> Option<CableTier> {
    let index = CABLES.iter().position(|(id, _)| *id == block)?;
    cable_tier(index)
}

/// The energy cell a block id names, with its tier's numbers; `None` for any other
/// block. The ids are the content's own (`data/powah/horizon/block/energy_cell_*`).
pub(crate) fn cell_of(block: &str) -> Option<CellTier> {
    let index = CELLS.iter().position(|(id, _)| *id == block)?;
    cell_tier(index)
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

/// A furnator's buffer (`AbstractEnergyProvider`): the tier's capacity, filled by
/// its own burn only, given out at the tier's transfer a side.
pub(crate) fn furnator_battery(tier: FurnatorTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: 0,
        max_extract: tier.transfer,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    }
}

/// The cell's buffer (`AbstractEnergyStorage`): the tier's capacity, its transfer a
/// side both ways.
pub(crate) fn cell_battery(tier: CellTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: tier.transfer,
        max_extract: tier.transfer,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// The buffer a block declares; the cable stores nothing (upstream's capacity is
/// 0), so it declares none and is only ever a relay.
pub(crate) fn battery_of(block: &str) -> Option<ModEnergyBuffer> {
    match Kind::of(block) {
        Some(Kind::Furnator(tier)) => Some(furnator_battery(tier)),
        Some(Kind::Cell(tier)) => Some(cell_battery(tier)),
        Some(Kind::Orb) => Some(orb_battery()),
        Some(Kind::Rod(tier)) => Some(rod_battery(tier)),
        _ => None,
    }
}

/// The furnator saved in a block entity's compound, clamped into its tier; a
/// fresh one is empty.
pub(crate) fn read(saved: &Tag, tier: FurnatorTier) -> Furnator {
    Furnator {
        energy: clamp_energy(int(saved, KEY_ENERGY), tier.capacity),
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

/// The bookmark this mod writes for "no recipe". Empty rather than absent because
/// a [`Compound`]'s keys are Java strings this module cannot name, so a removal
/// would need one.
const NO_RECIPE: &str = "";

/// A saved string, `None` when absent, of another type or the empty marker.
fn string(saved: &Tag, key: &str) -> Option<String> {
    match saved.get(key) {
        Some(Tag::String(value)) => {
            let value = value.to_string_lossy();
            (!value.is_empty()).then_some(value)
        }
        _ => None,
    }
}

/// A saved position (an int array of three), `None` when absent or malformed.
fn read_pos(saved: &Tag, key: &str) -> Option<[i32; 3]> {
    match saved.get(key) {
        Some(Tag::IntArray(pos)) if pos.len() == 3 => Some([pos[0], pos[1], pos[2]]),
        _ => None,
    }
}

/// Write an int under `key`.
fn write_int(saved: &mut Tag, key: &str, value: i32) {
    if let Tag::Compound(nbt) = saved {
        nbt.insert(key.into(), Tag::Int(value));
    }
}

/// Write a string under `key`.
fn write_string(saved: &mut Tag, key: &str, value: &str) {
    if let Tag::Compound(nbt) = saved {
        nbt.insert(key.into(), Tag::String(value.into()));
    }
}

/// The orb's state as its tick and `/powah info` read it.
pub(crate) struct OrbState {
    pub energy: i32,
    /// The input slots' item ids, in slot order.
    pub inputs: Vec<String>,
    pub recipe: Option<&'static EnergizingRecipe>,
}

/// The filled input slots of `items` (slot 0 is the result, 1..6 the inputs).
fn input_ids(host: &dyn Host, items: &[Option<mod_api::ItemStack>]) -> Vec<String> {
    items
        .iter()
        .skip(1)
        .flatten()
        .map(|stack| stack.item().id(host))
        .collect()
}

/// A loaded orb's state: its buffer and the recipe its six input slots match.
pub(crate) fn read_orb(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<OrbState> {
    let entity = host.block_entity(level, pos)?;
    let saved = Tag::Compound(entity.data(&*host)?);
    let mut inputs = Vec::new();
    for slot in 1..ORB_SLOTS as u32 {
        // A slot the entity does not have yet (this pin grows a mod entity's
        // `items` with its container, but a freshly placed orb answers before its
        // first tick) reads as empty rather than as a missing orb.
        if let Ok(Some(stack)) = entity.item_slot(&*host, slot) {
            inputs.push(stack.item().id(&*host));
        }
    }
    let recipe = {
        let ids: Vec<&str> = inputs.iter().map(String::as_str).collect();
        crate::runtime::energize(&ids)
    };
    Some(OrbState {
        energy: int(&saved, KEY_ENERGY),
        inputs,
        recipe,
    })
}

/// The orb's tick (`EnergizingOrbTile`): record the recipe its inputs match, keep
/// the buffer the rods fill, and produce when it reaches the recipe's energy. The
/// payload is left `Changed` only when something was written, so a waiting orb
/// does not dirty its chunk every tick and a rod's fill survives it.
fn orb_tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) -> bool {
    let resized = event.entity.items.len() != ORB_SLOTS;
    if resized {
        event.entity.items.resize(ORB_SLOTS, None);
    }
    let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
    let energy = int(&saved, KEY_ENERGY);
    let recorded = string(&saved, KEY_RECIPE);
    let inputs = input_ids(&*host, &event.entity.items);
    let ids: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let step = crate::runtime::orb_step(&ids, energy, recorded.as_deref());
    let mut changed = resized;

    match step.recipe {
        Some(recipe) if step.done => {
            // Upstream's `inv.clear()` then slot 0: the inputs become the result.
            let output = host
                .item_key(&Identifier::parse(recipe.output).expect("a registry id"))
                .and_then(|item| mod_api::ItemStack::new(item, recipe.count.unsigned_abs()));
            event.entity.items.fill(None);
            event.entity.items[ORB_OUTPUT_SLOT] = output;
            write_string(&mut saved, KEY_RECIPE, NO_RECIPE);
            changed = true;
        }
        Some(recipe) => {
            if recorded.as_deref() != Some(recipe.output) {
                write_string(&mut saved, KEY_RECIPE, recipe.output);
                changed = true;
            }
        }
        None => {
            if recorded.is_some() {
                write_string(&mut saved, KEY_RECIPE, NO_RECIPE);
                changed = true;
            }
        }
    }
    if step.energy != energy {
        write_int(&mut saved, KEY_ENERGY, step.energy);
        changed = true;
    }
    if let Tag::Compound(nbt) = saved {
        event.entity.nbt = nbt;
    }
    changed
}

/// One rod tick's move into its orb (`EnergizingRodTile.postTick`): find the orb,
/// warm up for 20 ticks while the orb has a recipe, then move
/// `min(stored, transfer)` into the orb's remaining room and take exactly what
/// the orb accepted. Answers whether anything was written.
fn rod_tick(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], tier: RodTier) -> bool {
    let Some(rod) = host.block_entity(level, pos) else {
        return false;
    };
    let Some(saved) = rod.data(&*host) else {
        return false;
    };
    let saved = Tag::Compound(saved);
    let mut patch = Compound::new();

    // The link: upstream sets it on placement and clears it when the orb goes, so
    // a missing, broken or out-of-range orb is searched for again here.
    let recorded = read_pos(&saved, KEY_ORB);
    let linked = recorded.filter(|orb| {
        in_range(pos, *orb)
            && block_at(&*host, level, *orb).as_deref().and_then(Kind::of) == Some(Kind::Orb)
    });
    let orb_pos = linked.or_else(|| find_orb(host, level, pos));
    if orb_pos != recorded {
        patch.insert(
            KEY_ORB.into(),
            Tag::IntArray(orb_pos.unwrap_or([0, 0, 0]).to_vec()),
        );
    }

    // A rod only gives to an orb that holds a recipe (`containRecipe`), and that
    // is what warms its 20-tick counter up; without one the counter cools.
    let awaiting = orb_pos.and_then(|orb| {
        let state = read_orb(host, level, orb)?;
        Some((orb, state.recipe?))
    });
    let cooldown = crate::runtime::rod_cooldown(awaiting.is_some(), int(&saved, KEY_COOLDOWN));
    if int(&saved, KEY_COOLDOWN) != cooldown {
        patch.insert(KEY_COOLDOWN.into(), Tag::Int(cooldown));
    }

    let mut applied = false;
    if let Some((orb, recipe)) = awaiting
        && cooldown >= crate::runtime::ROD_WARMUP
    {
        // Both sides are read again here, after every earlier move of this phase
        // has run, so a cable or a cell that touched either one is not undone.
        let rod_energy = rod
            .data(&*host)
            .map_or(0, |nbt| int(&Tag::Compound(nbt), KEY_ENERGY));
        let Some(orb_entity) = host.block_entity(level, orb) else {
            return false;
        };
        let Some(orb_saved) = orb_entity.data(&*host) else {
            return false;
        };
        let orb_energy = int(&Tag::Compound(orb_saved), KEY_ENERGY);
        let amount = crate::runtime::orb_fill(rod_energy, tier.transfer, orb_energy, recipe.energy);
        if amount > 0 {
            let mut rod_patch = Compound::new();
            rod_patch.insert(KEY_ENERGY.into(), Tag::Int(rod_energy - amount));
            let mut orb_patch = Compound::new();
            orb_patch.insert(KEY_ENERGY.into(), Tag::Int(orb_energy + amount));
            applied = host
                .transaction(|host, _tx| -> Result<bool, ()> {
                    rod.merge_data(host, rod_patch.clone()).map_err(|_| ())?;
                    orb_entity
                        .merge_data(host, orb_patch.clone())
                        .map_err(|_| ())?;
                    Ok(true)
                })
                .unwrap_or(false);
        }
    }

    if !patch.is_empty() {
        let _ = rod.merge_data(host, patch);
        return true;
    }
    applied
}

/// The nearest orb within `energizing_range` of `pos`, in `BlockPos.betweenClosed`
/// order (the order upstream's scan takes and keeps the first of).
fn find_orb(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<[i32; 3]> {
    for y in -crate::runtime::ENERGIZING_RANGE..=crate::runtime::ENERGIZING_RANGE {
        for x in -crate::runtime::ENERGIZING_RANGE..=crate::runtime::ENERGIZING_RANGE {
            for z in -crate::runtime::ENERGIZING_RANGE..=crate::runtime::ENERGIZING_RANGE {
                let candidate = [pos[0] + x, pos[1] + y, pos[2] + z];
                if candidate == pos || candidate == [0, 0, 0] {
                    continue;
                }
                if block_at(&*host, level, candidate)
                    .as_deref()
                    .and_then(Kind::of)
                    == Some(Kind::Orb)
                {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Whether `orb` stands within `energizing_range` of `rod` (Chebyshev, as
/// `BlockPos.betweenClosed` boxes it).
fn in_range(rod: [i32; 3], orb: [i32; 3]) -> bool {
    let range = crate::runtime::ENERGIZING_RANGE;
    (0..3).all(|axis| (rod[axis] - orb[axis]).abs() <= range)
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
        Kind::Furnator(tier) => {
            let fuel = match fuel {
                Some((item, count)) => format!("{item} x{count}"),
                None => "empty".to_owned(),
            };
            format!(
                "Furnator at {x},{y},{z}: {energy}/{} FE (comparator {}, \
                 carbon {carbon}, burning {}, fuel {fuel}, {} FE/t a side, t={time})",
                tier.capacity,
                comparator_strength(energy, tier.capacity),
                carbon > 0,
                tier.transfer,
            )
        }
        Kind::Cell(tier) => format!(
            "Energy Cell at {x},{y},{z}: {energy}/{} FE (comparator {}, {} FE/t a side, t={time})",
            tier.capacity,
            comparator_strength(energy, tier.capacity),
            tier.transfer,
        ),
        Kind::Cable(tier) => format!(
            "Energy Cable at {x},{y},{z}: no storage, moves up to {} FE/t (t={time})",
            tier.transfer
        ),
        // The orb and a rod have readouts of their own ([`orb_line`], [`rod_line`]);
        // [`info_line`] gives them those, so these arms are only the type's
        // exhaustiveness.
        Kind::Orb => orb_line(pos, energy, None, 0, time),
        Kind::Rod(tier) => rod_line(pos, energy, tier, None, time),
    }
}

/// The energizing rod tier a block id names, with its numbers; `None` for any
/// other block. The ids are the content's own
/// (`data/powah/horizon/block/energizing_rod_*`).
pub(crate) fn rod_of(block: &str) -> Option<RodTier> {
    let index = RODS.iter().position(|(id, _)| *id == block)?;
    rod_tier(index)
}

/// The orb's container: seven slots, slot 0 the result and 1..6 the inputs
/// (`this.inv.set(7)`, slot limit 1 on the inputs). Automation may fill any
/// input slot and take from the result slot, as upstream's `IInventoryHolder`
/// allows; there is no menu (no GUI in this port).
pub(crate) fn orb_container() -> ModContainer {
    ModContainer {
        slots: ORB_SLOTS,
        faces: std::array::from_fn(|_| (0..ORB_SLOTS).collect()),
        inputs: (1..ORB_SLOTS).collect(),
        outputs: vec![ORB_OUTPUT_SLOT],
        menu: None,
    }
}

/// The orb's buffer: it takes FE and gives none (`fillEnergy`), up to the largest
/// energizing recipe.
pub(crate) fn orb_battery() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ORB_MAX_ENERGY,
        max_insert: ORB_MAX_ENERGY,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

/// A rod's buffer (`AbstractEnergyStorage`): its tier's capacity, taking and
/// giving its tier's transfer a side. Upstream lets a rod reach FE only on the
/// face its `facing` state names; a declaration cannot vary by state, so every
/// face is open (recorded as a deviation in `notes.md`).
pub(crate) fn rod_battery(tier: RodTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: tier.transfer,
        max_extract: tier.transfer,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// The orb's readout: its buffer, the recipe its inputs match (or none) and how
/// many input slots are filled.
pub(crate) fn orb_line(
    pos: [i32; 3],
    energy: i32,
    recipe: Option<&'static EnergizingRecipe>,
    inputs: usize,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    match recipe {
        Some(recipe) => format!(
            "Energizing Orb at {x},{y},{z}: {energy}/{} FE (recipe {} x{}, inputs {inputs}, \
             t={time})",
            recipe.energy, recipe.output, recipe.count
        ),
        None => format!("Energizing Orb at {x},{y},{z}: 0/0 FE (no recipe, t={time})"),
    }
}

/// A rod's readout: its buffer, its tier's transfer and the orb it is linked to.
pub(crate) fn rod_line(
    pos: [i32; 3],
    energy: i32,
    tier: RodTier,
    orb: Option<[i32; 3]>,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let orb = match orb {
        Some([ox, oy, oz]) => format!("orb {ox},{oy},{oz}"),
        None => "no orb".to_owned(),
    };
    format!(
        "Energizing Rod at {x},{y},{z}: {energy}/{} FE ({} FE/t a side, {orb}, t={time})",
        tier.capacity, tier.transfer
    )
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
        Kind::Furnator(tier) => {
            let entity = host.block_entity(level, pos)?;
            let state = read(&Tag::Compound(entity.data(&*host)?), tier);
            let fuel = entity
                .item_slot(&*host, FUEL_SLOT as u32)
                .ok()
                .flatten()
                .map(|stack| (stack.item().id(&*host), stack.count()));
            (state.energy, state.carbon, fuel)
        }
        Kind::Cell(tier) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            (
                clamp_energy(int(&saved, KEY_ENERGY), tier.capacity),
                0,
                None,
            )
        }
        Kind::Cable(_) => (0, 0, None),
        Kind::Orb => {
            let orb = read_orb(host, level, pos)?;
            return Some(orb_line(
                pos,
                clamp_energy(orb.energy, orb.recipe.map_or(0, |recipe| recipe.energy)),
                orb.recipe,
                orb.inputs.len(),
                time,
            ));
        }
        Kind::Rod(tier) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            let orb = read_pos(&saved, KEY_ORB);
            return Some(rod_line(
                pos,
                clamp_energy(int(&saved, KEY_ENERGY), tier.capacity),
                tier,
                orb,
                time,
            ));
        }
    };
    let fuel = fuel.as_ref().map(|(item, count)| (item.as_str(), *count));
    Some(status_line(kind, pos, energy, carbon, fuel, time))
}

/// Set a furnator's, a cell's, the orb's or a rod's stored FE (`/powah set`): the
/// harness control the in-game check uses. Writing an `energy` key into a foreign
/// block's compound would be a foreign key in someone else's data, so only this
/// mod's blocks take it.
pub(crate) fn set_energy(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> SetOutcome {
    let Some(level) = level_now(host) else {
        return SetOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return SetOutcome::NoBlock;
    };
    let capacity = match Kind::of(&block) {
        Some(Kind::Furnator(tier)) => tier.capacity,
        Some(Kind::Cell(tier)) => tier.capacity,
        Some(Kind::Rod(tier)) => tier.capacity,
        // The orb's buffer is the recipe's own energy: with no recipe it stores
        // nothing, exactly as upstream's `onSlotChanged` leaves it.
        Some(Kind::Orb) => read_orb(host, level, pos)
            .and_then(|orb| orb.recipe)
            .map_or(0, |recipe| recipe.energy),
        Some(Kind::Cable(_)) => return SetOutcome::Cable,
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

/// A furnator tier: a block entity that burns its fuel and ticks at its own
/// generation into its own buffer.
pub(crate) struct FurnatorPart {
    queue: Arc<Mutex<Queue>>,
    tier: FurnatorTier,
}

impl FurnatorPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>, tier: FurnatorTier) -> FurnatorPart {
        FurnatorPart { queue, tier }
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
        let changed = burn(host, event, self.tier);
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
/// queues itself for the level tick's end, where the moves run. An energizing rod
/// is the same shape: it queues itself, and its move into the orb is one of the
/// level tick's jobs.
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

/// The orb: a block entity with the seven slots and the recipe-gated buffer, which
/// ticks to fill itself and to turn a full buffer into its result.
pub(crate) struct OrbPart;

#[mod_api::horizon_part]
impl BlockPart for OrbPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for OrbPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the recipe the inputs match, the buffer's value and the
    /// result, as `EnergizingOrbTile` decides them.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if !orb_tick(host, event) {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

/// The furnator's tick: `initBurn` (`AbstractGeneratorBlock`), one tick of
/// production, and the state written back. Answers whether anything changed.
fn burn(
    host: &mut dyn ServerHost,
    event: &mut BlockEntityTickEvent<'_>,
    tier: FurnatorTier,
) -> bool {
    let resized = event.entity.items.len() != SLOT_COUNT;
    if resized {
        event.entity.items.resize(SLOT_COUNT, None);
    }
    let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
    let before = read(&saved, tier);
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
    let produced = state.tick(tier) > 0;
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
    // The cables already covered by a network this tick: every cable in a network
    // ticks, so without this a run of N cables would plan the same relay N times.
    let mut planned: BTreeSet<[i32; 3]> = BTreeSet::new();
    for pos in queued {
        let Some(kind) = block_at(&*host, level, pos).as_deref().and_then(Kind::of) else {
            continue;
        };
        match kind {
            Kind::Furnator(tier) => push_out(host, level, pos, tier.transfer),
            Kind::Cell(tier) => push_out(host, level, pos, tier.transfer),
            // A rod gives to its orb in the same phase; the orb itself moves
            // nothing (it has no neighbours to give to).
            Kind::Rod(tier) => {
                rod_tick(host, level, pos, tier);
            }
            Kind::Orb => {}
            Kind::Cable(tier) => {
                if planned.contains(&pos) {
                    continue;
                }
                planned.extend(relay_network(host, level, pos, tier));
            }
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

/// The largest cable network one tick walks. Upstream's `CableNet` is unbounded;
/// this cap keeps one very long run from costing a tick, and is far past any
/// pack's cable run (recorded as a deviation from upstream's unbounded walk).
const NETWORK_MAX_CABLES: usize = 1024;

/// One external energy side of a cable network: the block the relay reaches and
/// the side of the network cable it hangs off, so the face the capability is read
/// and moved on is `cable_side.opposite()`.
#[derive(Clone, Copy)]
struct Port {
    pos: [i32; 3],
    cable_side: Direction,
}

/// The cables one network holds: every same-tier cable reachable from `start`
/// through cables only, in position order. A network stops at the first block
/// that is not a cable, which is what makes a run of cables relay while a wall
/// of stone does not.
fn discover_cables(
    host: &mut dyn ServerHost,
    level: LevelRef,
    start: [i32; 3],
    tier: CableTier,
) -> BTreeSet<[i32; 3]> {
    let mut cables = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(pos) = queue.pop_front() {
        for side in Direction::ALL {
            if cables.len() >= NETWORK_MAX_CABLES {
                break;
            }
            let next = offset(pos, side);
            if cables.contains(&next) {
                continue;
            }
            // Upstream's `CableNet` joins cables of one tier only: a basic run
            // does not relay into a starter one (`different tiers never connect`).
            if block_at(&*host, level, next).as_deref().and_then(Kind::of)
                == Some(Kind::Cable(tier))
            {
                cables.insert(next);
                queue.push_back(next);
            }
        }
    }
    cables
}

/// A cable's tick: gather the whole network, then plan and run its moves once.
/// Upstream's `CableNet` forwards one insertion through every connected cable of
/// the same tier; the port pulls, so one plan for the whole network per tick is
/// the same relay — a run of two or more cables now moves FE, where slice 1
/// moved it only between the block entities beside one cable.
///
/// Answers the network it covered, so the level tick does not plan it again.
fn relay_network(
    host: &mut dyn ServerHost,
    level: LevelRef,
    start: [i32; 3],
    tier: CableTier,
) -> BTreeSet<[i32; 3]> {
    let cables = discover_cables(host, level, start, tier);
    let mut ports: Vec<Neighbour> = Vec::new();
    let mut places: Vec<Port> = Vec::new();
    for cable in &cables {
        for side in Direction::ALL {
            let pos = offset(*cable, side);
            let Some(block) = block_at(&*host, level, pos) else {
                continue;
            };
            if matches!(Kind::of(&block), Some(Kind::Cable(_))) {
                continue; // inside the network: neither a source nor a sink
            }
            // A block touching two cables of one network is one place: reading it
            // through both cables would plan the same energy twice.
            if places.iter().any(|place| place.pos == pos) {
                continue;
            }
            let Some(buffer) = neighbour(host, level, pos, side.opposite()) else {
                continue;
            };
            ports.push(buffer);
            places.push(Port {
                pos,
                cable_side: side,
            });
        }
    }
    let rotation = level.game_time(&*host).unwrap_or(0).max(0) as usize;
    for step in network_moves(&ports, tier.transfer, rotation) {
        let from = places[step.from];
        let to = places[step.to];
        let want = u32::try_from(step.amount).unwrap_or(0);
        let _ = host.transaction(|host, tx| -> Result<u32, ()> {
            let Some(source) = level.capability(
                &mut *host,
                &ENERGY,
                from.pos,
                Some(from.cable_side.opposite()),
            ) else {
                return Ok(0);
            };
            let Some(target) =
                level.capability(&mut *host, &ENERGY, to.pos, Some(to.cable_side.opposite()))
            else {
                return Ok(0);
            };
            move_energy(host, &source, &target, want, tx).map_err(|_| ())
        });
    }
    cables
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
    use crate::runtime::{CELL_CAPACITY, CELL_TRANSFER, FURNATOR_CAPACITY, FURNATOR_TRANSFER};

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
    fn every_energy_cell_tier_is_one_of_the_ladder() {
        let ids = [
            ("powah:energy_cell_starter", 1_000_000, 1_000),
            ("powah:energy_cell_basic", 4_000_000, 4_000),
            ("powah:energy_cell_hardened", 10_000_000, 10_000),
            ("powah:energy_cell_blazing", 40_000_000, 40_000),
            ("powah:energy_cell_niotic", 100_000_000, 100_000),
            ("powah:energy_cell_spirited", 400_000_000, 400_000),
            ("powah:energy_cell_nitro", 2_000_000_000, 2_000_000),
        ];
        for (index, (id, capacity, transfer)) in ids.into_iter().enumerate() {
            let cell = cell_of(id).unwrap_or_else(|| panic!("{id} is an energy cell"));
            assert_eq!(
                (cell.capacity, cell.transfer),
                (capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                cell,
                cell_tier(index).expect("the same tier in the rule's table")
            );
        }
        assert_eq!(
            cell_of("powah:energy_cell_creative"),
            None,
            "the creative cell is not carried"
        );
        assert_eq!(cell_of(FURNATOR), None, "a furnator is not a cell");
        assert_eq!(cell_of("minecraft:chest"), None);
    }

    #[test]
    fn a_block_id_maps_to_its_machine_and_nothing_else_does() {
        assert_eq!(
            Kind::of(FURNATOR),
            Some(Kind::Furnator(furnator_tier(0).expect("the starter tier")))
        );
        assert_eq!(
            Kind::of(CELL),
            Some(Kind::Cell(cell_tier(0).expect("the starter tier")))
        );
        assert_eq!(
            Kind::of(CABLE),
            Some(Kind::Cable(cable_tier(0).expect("the starter tier")))
        );
        assert_eq!(Kind::of("minecraft:chest"), None);
        assert_eq!(
            Kind::of("powah:furnator_basic"),
            Some(Kind::Furnator(furnator_tier(1).expect("the basic tier"))),
            "a later furnator tier"
        );
        assert_eq!(
            Kind::of("powah:energy_cable_basic"),
            Some(Kind::Cable(cable_tier(1).expect("the basic cable"))),
            "a later cable tier"
        );
    }

    #[test]
    fn every_furnator_tier_is_one_of_the_ladder_with_its_own_buffer() {
        let expected = [
            ("powah:furnator_starter", 20, 20_000, 80),
            ("powah:furnator_basic", 80, 80_000, 320),
            ("powah:furnator_hardened", 200, 200_000, 800),
            ("powah:furnator_blazing", 800, 800_000, 3_200),
            ("powah:furnator_niotic", 2_000, 2_000_000, 8_000),
            ("powah:furnator_spirited", 8_000, 8_000_000, 32_000),
            ("powah:furnator_nitro", 40_000, 40_000_000, 160_000),
        ];
        for (index, (id, generation, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = furnator_of(id).unwrap_or_else(|| panic!("{id} is a furnator"));
            assert_eq!(
                (tier.generation, tier.capacity, tier.transfer),
                (generation, capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                tier,
                furnator_tier(index).expect("the same tier in the rule's table")
            );
            let buffer = battery_of(id).unwrap_or_else(|| panic!("{id} declares a buffer"));
            assert_eq!(buffer.capacity, capacity, "{id}'s capacity");
            assert_eq!(buffer.max_insert, 0, "{id} makes its own FE");
            assert_eq!(buffer.max_extract, transfer, "{id}'s extract cap");
            assert_eq!(buffer.insert_faces, [false; 6], "{id} takes no FE in");
            assert_eq!(
                buffer.extract_faces, [true; 6],
                "{id} gives FE on every face"
            );
        }
        assert_eq!(furnator_of(CELL), None, "a cell is not a furnator");
        assert_eq!(furnator_of("minecraft:furnace"), None);
    }

    #[test]
    fn every_cable_tier_is_one_of_the_ladder_and_declares_no_buffer() {
        let expected = [
            ("powah:energy_cable_starter", 500),
            ("powah:energy_cable_basic", 2_000),
            ("powah:energy_cable_hardened", 5_000),
            ("powah:energy_cable_blazing", 20_000),
            ("powah:energy_cable_niotic", 50_000),
            ("powah:energy_cable_spirited", 200_000),
            ("powah:energy_cable_nitro", 1_000_000),
        ];
        for (index, (id, transfer)) in expected.into_iter().enumerate() {
            let tier = cable_of(id).unwrap_or_else(|| panic!("{id} is an energy cable"));
            assert_eq!(tier.transfer, transfer, "{id} is the ladder's tier {index}");
            assert_eq!(
                tier,
                cable_tier(index).expect("the same tier in the rule's table")
            );
            assert_eq!(battery_of(id), None, "{id} stores nothing");
        }
        assert_eq!(cable_of(FURNATOR), None, "a furnator is not a cable");
        assert_eq!(cable_of("minecraft:chest"), None);
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
        let buffer = furnator_battery(furnator_tier(0).expect("the starter tier"));
        assert_eq!(buffer.capacity, FURNATOR_CAPACITY);
        assert_eq!(buffer.max_insert, 0, "it makes its own FE");
        assert_eq!(buffer.max_extract, FURNATOR_TRANSFER);
        assert_eq!(buffer.insert_faces, [false; 6]);
        assert_eq!(buffer.extract_faces, [true; 6]);
    }

    #[test]
    fn the_cell_declares_a_buffer_both_ways() {
        let starter = cell_tier(0).expect("the starter tier");
        let buffer = cell_battery(starter);
        assert_eq!(buffer.capacity, CELL_CAPACITY);
        assert_eq!(buffer.max_insert, CELL_TRANSFER);
        assert_eq!(buffer.max_extract, CELL_TRANSFER);
        assert_eq!(buffer.insert_faces, [true; 6]);
        assert_eq!(buffer.extract_faces, [true; 6]);
        assert_eq!(battery_of(CELL), Some(buffer));
    }

    #[test]
    fn every_cell_tier_declares_its_own_buffer_and_nothing_else_does() {
        for (index, (block, _)) in CELLS.iter().enumerate() {
            let tier = cell_tier(index).expect("a tier on the ladder");
            assert_eq!(
                battery_of(block),
                Some(cell_battery(tier)),
                "{block} declares tier {index}'s numbers"
            );
        }
        assert_eq!(battery_of("powah:energy_cell_creative"), None);
    }

    #[test]
    fn the_cable_declares_no_buffer_because_it_stores_nothing() {
        assert_eq!(battery_of(CABLE), None);
        assert_eq!(battery_of("minecraft:chest"), None);
    }

    #[test]
    fn a_fresh_compound_is_an_empty_furnator() {
        assert_eq!(
            read(
                &Tag::Compound(Compound::new()),
                furnator_tier(0).expect("the starter tier")
            ),
            Furnator::new()
        );
    }

    #[test]
    fn the_tick_s_state_reads_back_from_the_compound_it_wrote() {
        let mut saved = Tag::Compound(Compound::new());
        let state = Furnator {
            energy: 20_000,
            carbon: 46_800,
        };
        write(&mut saved, &state);
        assert_eq!(
            read(&saved, furnator_tier(0).expect("the starter tier")),
            state
        );
        assert_eq!(
            saved.get(KEY_ENERGY),
            Some(&Tag::Int(20_000)),
            "the buffer is the int `energy` the energy capability reads"
        );
        assert_eq!(saved.get(KEY_CARBON), Some(&Tag::Int(46_800)));
    }

    #[test]
    fn every_energizing_rod_tier_is_one_of_the_ladder_with_its_own_buffer() {
        let expected = [
            ("powah:energizing_rod_starter", 10_000, 100),
            ("powah:energizing_rod_basic", 40_000, 400),
            ("powah:energizing_rod_hardened", 100_000, 1_000),
            ("powah:energizing_rod_blazing", 400_000, 4_000),
            ("powah:energizing_rod_niotic", 1_000_000, 10_000),
            ("powah:energizing_rod_spirited", 4_000_000, 40_000),
            ("powah:energizing_rod_nitro", 20_000_000, 200_000),
        ];
        for (index, (id, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = rod_of(id).unwrap_or_else(|| panic!("{id} is an energizing rod"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                tier,
                rod_tier(index).expect("the same tier in the rule's table")
            );
            let buffer = battery_of(id).unwrap_or_else(|| panic!("{id} declares a buffer"));
            assert_eq!(buffer.capacity, capacity, "{id}'s capacity");
            assert_eq!(buffer.max_insert, transfer, "{id}'s insert cap");
            assert_eq!(buffer.max_extract, transfer, "{id}'s extract cap");
            assert_eq!(
                buffer.insert_faces, [true; 6],
                "{id} takes FE on every face"
            );
            assert_eq!(
                buffer.extract_faces, [true; 6],
                "{id} gives FE on every face"
            );
        }
        assert_eq!(rod_of(ORB), None, "an orb is not a rod");
        assert_eq!(rod_of("powah:energy_cell_starter"), None);
    }

    #[test]
    fn the_orb_declares_a_seven_slot_container_and_a_take_only_buffer() {
        let container = orb_container();
        assert_eq!(container.slots, ORB_SLOTS, "slot 0 plus six inputs");
        assert_eq!(
            container.inputs,
            (1..ORB_SLOTS).collect::<Vec<_>>(),
            "automation fills the six input slots"
        );
        assert_eq!(
            container.outputs,
            vec![ORB_OUTPUT_SLOT],
            "automation takes the result slot"
        );
        assert_eq!(
            container.faces,
            std::array::from_fn(|_| (0..ORB_SLOTS).collect::<Vec<_>>()),
            "every face reaches all seven slots"
        );
        assert_eq!(container.menu, None, "this port has no GUI");

        let buffer = orb_battery();
        assert_eq!(buffer.capacity, ORB_MAX_ENERGY);
        assert_eq!(
            buffer.max_insert, ORB_MAX_ENERGY,
            "upstream's transfer is its capacity"
        );
        assert_eq!(buffer.max_extract, 0, "the orb never gives FE back");
        assert_eq!(buffer.insert_faces, [true; 6]);
        assert_eq!(buffer.extract_faces, [false; 6]);
        assert_eq!(battery_of(ORB), Some(buffer));
    }

    #[test]
    fn the_orb_and_the_rod_print_the_lines_the_e2e_asserts() {
        let steel = crate::runtime::energizing()
            .iter()
            .find(|recipe| recipe.output == "powah:steel_energized")
            .expect("the steel recipe");
        assert_eq!(
            orb_line([0, 100, 20], 0, Some(steel), 2, 120),
            "Energizing Orb at 0,100,20: 0/10000 FE (recipe powah:steel_energized x2, inputs 2, \
             t=120)"
        );
        assert_eq!(
            orb_line([0, 100, 20], 0, None, 0, 40),
            "Energizing Orb at 0,100,20: 0/0 FE (no recipe, t=40)"
        );
        let starter = rod_tier(0).expect("the starter rod");
        assert_eq!(
            rod_line([0, 101, 20], 10_000, starter, Some([0, 100, 20]), 120),
            "Energizing Rod at 0,101,20: 10000/10000 FE (100 FE/t a side, orb 0,100,20, t=120)"
        );
        let nitro = rod_tier(6).expect("the nitro rod");
        assert_eq!(
            rod_line([0, 101, 20], 0, nitro, None, 40),
            "Energizing Rod at 0,101,20: 0/20000000 FE (200000 FE/t a side, no orb, t=40)"
        );
    }

    #[test]
    fn the_info_lines_are_the_ones_the_e2e_asserts() {
        let starter_furnator = Kind::Furnator(furnator_tier(0).expect("the starter tier"));
        assert_eq!(
            status_line(starter_furnator, [0, 100, 0], 0, 0, None, 150),
            "Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, \
             80 FE/t a side, t=150)"
        );
        assert_eq!(
            status_line(
                starter_furnator,
                [0, 100, 0],
                320,
                46_800,
                Some(("minecraft:coal", 4)),
                282
            ),
            "Furnator at 0,100,0: 320/20000 FE (comparator 0, carbon 46800, burning true, fuel \
             minecraft:coal x4, 80 FE/t a side, t=282)"
        );
        let nitro_furnator = Kind::Furnator(furnator_tier(6).expect("the nitro tier"));
        assert_eq!(
            status_line(nitro_furnator, [0, 100, 8], 40_000_000, 8_000, None, 500),
            "Furnator at 0,100,8: 40000000/40000000 FE (comparator 15, carbon 8000, burning \
             true, fuel empty, 160000 FE/t a side, t=500)"
        );
        let starter = Kind::Cell(cell_tier(0).expect("the starter tier"));
        assert_eq!(
            status_line(starter, [2, 100, 0], 1_360, 0, None, 290),
            "Energy Cell at 2,100,0: 1360/1000000 FE (comparator 0, 1000 FE/t a side, t=290)"
        );
        assert_eq!(
            status_line(starter, [2, 100, 0], 1_000_000, 0, None, 380),
            "Energy Cell at 2,100,0: 1000000/1000000 FE (comparator 15, 1000 FE/t a side, t=380)"
        );
        let nitro = Kind::Cell(cell_tier(6).expect("the nitro tier"));
        assert_eq!(
            status_line(nitro, [2, 100, 0], 2_000_000_000, 0, None, 380),
            "Energy Cell at 2,100,0: 2000000000/2000000000 FE (comparator 15, 2000000 FE/t a \
             side, t=380)"
        );
        assert_eq!(
            status_line(
                Kind::Cable(cable_tier(0).expect("the starter cable")),
                [1, 100, 0],
                0,
                0,
                None,
                188
            ),
            "Energy Cable at 1,100,0: no storage, moves up to 500 FE/t (t=188)"
        );
        assert_eq!(
            status_line(
                Kind::Cable(cable_tier(1).expect("the basic cable")),
                [1, 100, 0],
                0,
                0,
                None,
                188
            ),
            "Energy Cable at 1,100,0: no storage, moves up to 2000 FE/t (t=188)"
        );
    }
}
