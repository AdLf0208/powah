// The three blocks tick on the server.
fn observe_block_entity_ticks(block: &str, ticks: &mut bool) {
    crate::game::powah_state::on_block_entity_ticks(block, ticks);
}
