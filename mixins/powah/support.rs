// Game-side glue for Powah slice 1 (Furnator, Energy Cell, Energy Cable).
//
// Injected into `crate::game` as `powah_state`; it needs the game, so it is not
// part of the standalone library (only `runtime.rs` is).
//
// Energy: the furnator and the cell declare a `Capabilities.Energy.BLOCK`
// buffer (API 142) whose amount is the int `energy` in the block entity's
// saved compound, as upstream's `AbstractEnergyStorage` keeps it. The cable has
// no buffer at all (upstream's capacity is 0), so it moves energy between the
// block entities beside it on its own tick.
//
// A block entity is out of the world during its own tick, so no block can read
// a neighbour's buffer there: every block queues its position and the moves run
// at `level_tick_post`, where all of them are back in the world. That is also
// where `native_move_energy` (`EnergyHandlerUtil.move`) is usable.
//
// Fuel: the burn time is the engine's `minecraft:cooking_fuel` value
// (`Providers::burn_time`, the counterpart of `stack.getBurnTime(SMOKING)`), so
// no fuel table is invented.

use std::sync::Mutex;

use horizon_client::containers::{BlockEntity, ModBlockEntity, ModContainer, ModEnergyBuffer};
use horizon_client::items::ItemStack;
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use crate::game::powah::{
    CABLE_TRANSFER, CELL_CAPACITY, CELL_TRANSFER, FURNATOR_CAPACITY, FURNATOR_TRANSFER, Furnator,
    Neighbour, cable_moves, clamp_energy, comparator_strength, push_amount,
};

const FURNATOR: &str = "powah:furnator_starter";
const CELL: &str = "powah:energy_cell_starter";
const CABLE: &str = "powah:energy_cable_starter";
/// The furnator's fuel slot (upstream's slot 1 of 2; the charging slot is a gap).
const FUEL_SLOT: usize = 0;

fn nbt_key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&nbt_key(key)) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(nbt_key(key), Tag::Int(value));
}

fn state_name(
    game: &crate::game::Game<'_>,
    state: horizon_world::registry::BlockStateId,
) -> String {
    game.res.facts().get(state).name.to_string()
}

fn block_at(game: &crate::game::Game<'_>, pos: [i32; 3]) -> String {
    state_name(game, game.world.block(pos[0], pos[1], pos[2]))
}

/// `BlockEntity.setChanged`: the chunk is written at the next save.
fn mark_modified(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
}

fn furnator_state(entity: &ModBlockEntity) -> Furnator {
    Furnator {
        energy: clamp_energy(read_int(&entity.nbt, "energy"), FURNATOR_CAPACITY),
        carbon: read_int(&entity.nbt, "carbon").max(0),
    }
}

fn store_furnator(entity: &mut ModBlockEntity, state: &Furnator) {
    write_int(&mut entity.nbt, "energy", state.energy);
    write_int(&mut entity.nbt, "carbon", state.carbon);
}

/// The engine's fuel value (`minecraft:cooking_fuel.burn_time`), 0 if none.
fn fuel_burn_time(game: &crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    let providers = game.res.providers();
    let items = game.res.items();
    providers.burn_time(items, stack, false).max(0) as u32
}

/// The buffers: upstream's `AbstractEnergyProvider` (extract only, 80 a side)
/// and `AbstractEnergyStorage` (both ways, 1,000 a side). The cable declares
/// none: it stores nothing.
#[allow(private_interfaces)]
pub fn on_block_entity_energy(block: &str, energy: &mut Option<ModEnergyBuffer>) {
    *energy = match block {
        FURNATOR => Some(ModEnergyBuffer {
            capacity: FURNATOR_CAPACITY,
            max_insert: 0,
            max_extract: FURNATOR_TRANSFER,
            insert_faces: [false; 6],
            extract_faces: [true; 6],
        }),
        CELL => Some(ModEnergyBuffer {
            capacity: CELL_CAPACITY,
            max_insert: CELL_TRANSFER,
            max_extract: CELL_TRANSFER,
            insert_faces: [true; 6],
            extract_faces: [true; 6],
        }),
        _ => None,
    };
}

/// The three blocks have a block entity (`native_block_entity`, API 15).
#[allow(private_interfaces)]
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == FURNATOR || block == CELL || block == CABLE {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// ... and tick (`native_block_entity_ticks`, API 15).
#[allow(private_interfaces)]
pub fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if block == FURNATOR || block == CELL || block == CABLE {
        *ticks = true;
    }
}

/// The furnator's fuel slot (`native_block_entity_container`, API 16).
/// Upstream has two slots (0 charging, 1 fuel); slice 1 keeps the fuel one and
/// records the charging slot as a gap, so the slot index is 0, not 1.
#[allow(private_interfaces)]
pub fn on_block_entity_container(block: &str, container: &mut Option<ModContainer>) {
    if block != FURNATOR {
        return;
    }
    *container = Some(ModContainer {
        slots: 1,
        faces: std::array::from_fn(|_| vec![0]),
        inputs: vec![0],
        outputs: vec![0],
        menu: None,
    });
}

/// One server tick of a Powah block (`native_block_entity_tick`, API 17).
#[allow(private_interfaces)]
pub fn on_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
) {
    if entity.id == FURNATOR {
        tick_furnator(game, pos, entity);
    }
    // The cell and the cable store nothing of their own: they only queue a
    // position for the move phase below.
    queue(game, pos);
}

/// The furnator: load fuel when upstream would, burn one tick of carbon and
/// store what fits.
fn tick_furnator(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    let mut state = furnator_state(entity);
    if state.wants_fuel()
        && let Some(stack) = entity.items.first().and_then(Option::clone)
    {
        let burn = fuel_burn_time(game, &stack);
        if state.load_fuel(burn) {
            consume_fuel(game, entity, &stack);
            mark_modified(game, pos);
        }
    }
    if state.tick() > 0 {
        mark_modified(game, pos);
    }
    store_furnator(entity, &state);
}

/// `ItemStack.shrink` + `getCraftingRemainingItem`: one item is consumed and a
/// remainder (a lava bucket's bucket) takes its place when the stack runs out.
fn consume_fuel(game: &crate::game::Game<'_>, entity: &mut ModBlockEntity, stack: &ItemStack) {
    let remainder = game
        .res
        .items()
        .get(&stack.item)
        .and_then(|info| info.remainder.clone());
    let left = stack.count - 1;
    entity.items[FUEL_SLOT] = if left > 0 {
        let mut kept = stack.clone();
        kept.count = left;
        Some(kept)
    } else {
        remainder.map(|item| ItemStack::new(item.as_str(), 1))
    };
}

/// Positions queued for this tick's energy moves, by dimension.
static QUEUE: Mutex<Vec<(String, [i32; 3])>> = Mutex::new(Vec::new());

fn queue(game: &crate::game::Game<'_>, pos: [i32; 3]) {
    let dimension = game.native_dimension().to_owned();
    let mut queue = QUEUE.lock().unwrap_or_else(|poison| poison.into_inner());
    if !queue
        .iter()
        .any(|(dim, at)| dim == &dimension && *at == pos)
    {
        queue.push((dimension, pos));
    }
}

fn offset(pos: [i32; 3], side: Direction) -> [i32; 3] {
    let delta = side.normal();
    [pos[0] + delta[0], pos[1] + delta[1], pos[2] + delta[2]]
}

/// After a level's tick: run the queued energy moves (`native_move_energy`,
/// API 142). Both blocks of every move are in the world here, unlike inside
/// their own ticks.
#[allow(private_interfaces)]
pub fn on_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    let queued = {
        let mut queue = QUEUE.lock().unwrap_or_else(|poison| poison.into_inner());
        std::mem::take(&mut *queue)
    };
    if queued.is_empty() {
        return;
    }
    let mut later = Vec::new();
    for (dim, pos) in queued {
        if dim != dimension {
            later.push((dim, pos));
            continue;
        }
        match block_at(game, pos).as_str() {
            FURNATOR => push_out(game, dimension, pos, FURNATOR_TRANSFER),
            CELL => push_out(game, dimension, pos, CELL_TRANSFER),
            CABLE => relay(game, dimension, pos),
            _ => {}
        }
    }
    if !later.is_empty() {
        QUEUE
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .extend(later);
    }
}

/// `AbstractEnergyStorage.extractFromSides`: at most the transfer and what is
/// stored, into every side whose neighbour can take it.
#[allow(private_interfaces)]
fn push_out(game: &mut crate::game::Game<'_>, dimension: &str, pos: [i32; 3], transfer: i32) {
    let Some(own) = game.native_energy(dimension, pos, Direction::Up) else {
        return;
    };
    let amount = push_amount(own.amount, transfer);
    if amount == 0 {
        return;
    }
    for side in Direction::ALL {
        let neighbour = offset(pos, side);
        let Some(target) = game.native_energy(dimension, neighbour, side.opposite()) else {
            continue;
        };
        if !target.can_insert {
            continue;
        }
        game.native_move_energy(
            dimension,
            pos,
            side,
            neighbour,
            side.opposite(),
            push_amount(amount, target.max_insert),
        );
    }
}

/// The cable's move: it reads each neighbour through the side facing it and
/// lets the pure planner decide where the energy goes (upstream's round-robin
/// is the rotation, seeded by the game time).
#[allow(private_interfaces)]
fn relay(game: &mut crate::game::Game<'_>, dimension: &str, pos: [i32; 3]) {
    let mut sides: [Option<Neighbour>; 6] = [None; 6];
    for (index, side) in Direction::ALL.into_iter().enumerate() {
        let neighbour = offset(pos, side);
        sides[index] = game
            .native_energy(dimension, neighbour, side.opposite())
            .map(|energy| Neighbour {
                energy: energy.amount,
                capacity: energy.capacity,
                max_insert: energy.max_insert,
                max_extract: energy.max_extract,
                can_insert: energy.can_insert,
                can_extract: energy.can_extract,
            });
    }
    let rotation = (game.world.game_time % 6) as usize;
    for step in cable_moves(&sides, CABLE_TRANSFER, rotation) {
        let from_side = Direction::ALL[step.from];
        let to_side = Direction::ALL[step.to];
        game.native_move_energy(
            dimension,
            offset(pos, from_side),
            from_side.opposite(),
            offset(pos, to_side),
            to_side.opposite(),
            step.amount,
        );
    }
}

/// `/powah info <x> <y> <z>` (`register_commands`, API 8).
#[allow(private_interfaces)]
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("powah info <pos:block_pos>"));
    commands.push(NativeCommand::new(
        "powah set <amount:integer> <pos:block_pos>",
    ));
}

fn parse_pos(words: &[&str]) -> Option<[i32; 3]> {
    Some([
        words.first()?.parse().ok()?,
        words.get(1)?.parse().ok()?,
        words.get(2)?.parse().ok()?,
    ])
}

fn stack_line(stack: Option<&ItemStack>) -> String {
    match stack {
        Some(stack) => format!("{} x{}", stack.item, stack.count),
        None => "empty".to_owned(),
    }
}

/// A machine's status line. `t=` is the world game time, so the e2e can tell a
/// fresh reply from a stale one.
fn status_text(game: &crate::game::Game<'_>, pos: [i32; 3]) -> String {
    let key = (pos[0], pos[1], pos[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&key) else {
        return "No Powah block there.".to_owned();
    };
    match entity.id.as_str() {
        FURNATOR => {
            let state = furnator_state(entity);
            format!(
                "Furnator at {},{},{}: {}/{} FE (comparator {}, carbon {}, burning {}, fuel {}, t={})",
                pos[0],
                pos[1],
                pos[2],
                state.energy,
                FURNATOR_CAPACITY,
                comparator_strength(state.energy, FURNATOR_CAPACITY),
                state.carbon,
                state.burning(),
                stack_line(entity.items.first().and_then(Option::as_ref)),
                game.world.game_time
            )
        }
        CELL => {
            let energy = clamp_energy(read_int(&entity.nbt, "energy"), CELL_CAPACITY);
            format!(
                "Energy Cell at {},{},{}: {}/{} FE (comparator {}, t={})",
                pos[0],
                pos[1],
                pos[2],
                energy,
                CELL_CAPACITY,
                comparator_strength(energy, CELL_CAPACITY),
                game.world.game_time
            )
        }
        CABLE => format!(
            "Energy Cable at {},{},{}: no storage, moves up to {} FE/t (t={})",
            pos[0], pos[1], pos[2], CABLE_TRANSFER, game.world.game_time
        ),
        _ => "No Powah block there.".to_owned(),
    }
}

/// The `/powah` command body (`perform_command`, API 8). `info` reads, `set`
/// (a harness control) writes the stored FE of a furnator or a cell.
#[allow(private_interfaces)]
pub fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.as_slice() {
        ["powah", "info", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal("Usage: /powah info <x> <y> <z>"));
                return;
            };
            game.native_feedback(Text::literal(status_text(game, pos)));
        }
        ["powah", "set", amount, x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal("Usage: /powah set <amount> <x> <y> <z>"));
                return;
            };
            let Ok(amount) = amount.parse::<i32>() else {
                game.native_feedback(Text::literal("The amount must be a whole number of FE."));
                return;
            };
            let Some(after) = game.native_with_mod_block_entity(pos, |entity| {
                if entity.id == CABLE {
                    return None;
                }
                let capacity = if entity.id == FURNATOR {
                    FURNATOR_CAPACITY
                } else {
                    CELL_CAPACITY
                };
                let after = clamp_energy(amount, capacity);
                write_int(&mut entity.nbt, "energy", after);
                Some(after)
            }) else {
                game.native_feedback(Text::literal("No Powah block there."));
                return;
            };
            let Some(after) = after else {
                game.native_feedback(Text::literal("The energy cable stores no energy."));
                return;
            };
            mark_modified(game, pos);
            game.native_feedback(Text::literal(format!("Energy set to {after} FE.")));
        }
        _ => {}
    }
}

/// A world opened or closed: the queued positions are per world.
#[allow(private_interfaces)]
pub fn on_world_lifecycle(
    _game: &mut crate::game::Game<'_>,
    _phase: crate::game::native_lifecycle::WorldPhase,
) {
    QUEUE
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clear();
}
