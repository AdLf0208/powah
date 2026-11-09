// After the level's tick: the queued energy moves.
fn observe_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::powah_state::on_level_tick_post(game, dimension);
}
