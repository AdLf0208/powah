// Declare `/powah info|set`.
fn observe_register(commands: &mut Vec<NativeCommand>) {
    crate::game::powah_state::on_register_commands(commands);
}
