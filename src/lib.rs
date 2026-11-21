//! Powah for Horizon — slice 1, the starter Furnator, Energy Cell and Energy Cable.
//!
//! One flat SDK package (mod API 0.2): the carried blocks, their items, models,
//! textures, recipes, loot, tags and lang are the `assets/` and `data/` beside
//! `src/`, and the behaviour is the one `impl Hooks` below. The rules themselves
//! live in [`runtime`], byte-identical to the retired `.hnmod` bundle's
//! `runtime.rs`; [`powah`] is the glue.
//!
//! What the retired bundle expressed as nine injected helpers is here four shapes:
//!
//! - three **block entity types** declared in [`Hooks::register`]
//!   (`block_entity_type(..).blocks(..)`), with the furnator and the
//!   cell/cable each carrying a **Part** — a ticked machine, so the nine hooks it
//!   used (`block_entity`, `block_entity_ticks`, `block_entity_tick`) are one
//!   declaration and two Part methods;
//! - two **declarations**: the furnator's container (one fuel slot) and the energy
//!   buffers of the furnator and the cell (`Registrar::block_container`,
//!   `Registrar::block_energy`) — the old `block_entity_container` and
//!   `block_entity_energy` hooks, keyed by the block instead of a shared output;
//! - one **level hook**, [`Hooks::level_tick_post`]: the queued energy moves, since
//!   a block entity is out of the world during its own tick;
//! - the **typed command pair** [`Hooks::register_commands`] /
//!   [`Hooks::run_mod_command`], which replaces `register_commands` +
//!   `perform_command`, plus [`Hooks::world_lifecycle`], which drops the queue when
//!   a world opens or closes.
//!
//! Its `content_root` embedded-content helper has no successor: one `.hmod`
//! carries the content and the code, which is what that helper existed to work
//! around.
pub mod runtime;

mod powah;

use std::sync::{Arc, Mutex};

use mod_api::{
    self, Flow, Hooks, HostSide, LevelTickPostEvent, RegisterCommandsEvent, Registrar,
    RegistrationContext, RunModCommandEvent, ServerHost, Text, WorldLifecycleEvent,
};

use crate::powah::{Queue, SetOutcome};

/// Declare one idle block entity type (a cell or the cable) and attach the Part
/// that only queues its position for the level tick's move phase.
fn declare_idle(
    registrar: &mut dyn Registrar,
    context: &RegistrationContext,
    queue: &Arc<Mutex<Queue>>,
    name: &str,
) {
    let kind = powah::declare(registrar, name);
    let part = powah::IdlePart::new(Arc::clone(queue));
    if let Err(error) = registrar.block_entity_part(&kind, part) {
        context.refuse(&error.to_string());
    }
}

/// The mod's session state: the machines whose block entity ticked this tick,
/// per level, drained at the end of that level's tick.
#[derive(Default)]
struct Mod {
    queue: Arc<Mutex<Queue>>,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the machines: a block entity type each, the furnator's one fuel slot,
    /// its and every cell tier's FE buffer, and the Parts that make them tick.
    ///
    /// Nine blocks in all — the furnator, the seven energy cell tiers and the cable.
    /// The cable declares no buffer (it stores nothing; upstream's capacity is 0) and
    /// still needs a type and a tick, because a cable is what queues its own position
    /// for the level's move phase.
    fn register(&self, registrar: &mut dyn Registrar, context: &RegistrationContext) {
        let furnator = registrar.block_key(powah::FURNATOR);
        registrar.block_container(&furnator, powah::container());
        registrar.block_energy(&furnator, powah::furnator_battery());
        // Every energy cell tier: one buffer with the tier's own capacity and
        // per-side transfer (upstream's `AbstractEnergyStorage` over `EnergyConfig`).
        for (index, (block, _)) in powah::CELLS.into_iter().enumerate() {
            let tier = runtime::cell_tier(index).expect("every cell id is on the tier ladder");
            let cell = registrar.block_key(block);
            registrar.block_energy(&cell, powah::cell_battery(tier));
        }
        // The energizing orb: the seven slots and the insert-only buffer a rod fills.
        let orb = registrar.block_key(powah::ORB);
        registrar.block_container(&orb, powah::orb_container());
        registrar.block_energy(&orb, powah::orb_battery());
        // Every rod tier: its own capacity and per-side transfer.
        for (index, (block, _)) in powah::RODS.into_iter().enumerate() {
            let tier = runtime::rod_tier(index).expect("every rod id is on the tier ladder");
            let rod = registrar.block_key(block);
            registrar.block_energy(&rod, powah::rod_battery(tier));
        }

        // One type per block, and three Part types: the furnator burns, the orb
        // fills and completes, a cell, a cable and a rod only queue their position
        // for the move phase (the rod's move into its orb is one of its jobs).
        let furnator_kind = powah::declare(registrar, powah::FURNATOR_TYPE);
        let part = powah::FurnatorPart::new(Arc::clone(&self.queue));
        if let Err(error) = registrar.block_entity_part(&furnator_kind, part) {
            context.refuse(&error.to_string());
        }
        let orb_kind = powah::declare(registrar, powah::ORB_TYPE);
        if let Err(error) = registrar.block_entity_part(&orb_kind, powah::OrbPart) {
            context.refuse(&error.to_string());
        }
        for (_, name) in powah::CELLS {
            declare_idle(registrar, context, &self.queue, name);
        }
        for (_, name) in powah::RODS {
            declare_idle(registrar, context, &self.queue, name);
        }
        declare_idle(registrar, context, &self.queue, powah::CABLE_TYPE);
    }

    /// After the level's tick: run the queued moves, where every block entity is
    /// back in the world. A client's tick runs nothing.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        // The moves reach the machines through this level's `ENERGY` capability, and
        // `LevelRef::capability` answers only for the level the server ticks now.
        // Outside that window the queue is left alone and the next tick retries, so
        // no machine's move is dropped; the queue is bounded by the machine count.
        if !event.level.in_place(host) {
            return;
        }
        let queued = self
            .queue
            .lock()
            .expect("the powah queue lock is held for one drain")
            .take(event.level);
        powah::on_level_tick_post(host, event.level, queued);
    }

    /// A world opened or closed: queued positions belong to the world they were
    /// queued in, so none outlives it.
    fn world_lifecycle(&self, _host: &mut dyn ServerHost, _event: &mut WorldLifecycleEvent<'_>) {
        self.queue
            .lock()
            .expect("the powah queue lock is held for one clear")
            .clear();
    }

    /// `/powah info <pos>` and `/powah set <amount> <pos>`: operator-only
    /// diagnostics (level 2). Slice 1 has no GUI and no in-game FE readout, so
    /// these are the harness controls its in-game check uses; because they act
    /// on any loaded machine at chosen coordinates, the server refuses them to
    /// non-operators. Upstream's own commands are its config and GUI actions;
    /// none are ported.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        powah::declare_commands(event.commands);
    }

    /// The command bodies: the arguments are parsed and resolved by the server's
    /// dispatcher, so this only reads a machine and answers, in 0.1.0's words.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
        let pos = powah::block_pos(event.command.argument("pos"));
        let amount = powah::integer(event.command.argument("amount"));
        let line = match (literals.as_slice(), pos) {
            (["powah", "info"], Some(pos)) => {
                powah::info_line(host, pos).unwrap_or_else(|| "No Powah block there.".to_owned())
            }
            (["powah", "set"], Some(pos)) => {
                let Some(amount) = amount else {
                    return Flow::Continue;
                };
                match powah::set_energy(host, pos, amount) {
                    SetOutcome::Set(after) => format!("Energy set to {after} FE."),
                    SetOutcome::Cable => "The energy cable stores no energy.".to_owned(),
                    SetOutcome::NoBlock => "No Powah block there.".to_owned(),
                }
            }
            _ => return Flow::Continue,
        };
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }
}
