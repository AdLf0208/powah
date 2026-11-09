// The furnator's fuel slot.
fn observe_block_entity_container(
    block: &str,
    container: &mut Option<horizon_client::containers::ModContainer>,
) {
    crate::game::powah_state::on_block_entity_container(block, container);
}
