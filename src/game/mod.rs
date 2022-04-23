mod block;
mod block_buffer;
mod cam;
mod chunk;
mod map;
mod player;
mod render;

use bevy::prelude::*;

use crate::state::GameState;

use self::{
    cam::CamPlugin,
    chunk::ChunkPlugin,
    map::{Map, MapPlugin},
    player::PlayerPlugin,
    render::RenderPlugin,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CamPlugin)
            .add_plugin(ChunkPlugin)
            .add_plugin(MapPlugin)
            .add_plugin(PlayerPlugin)
            .add_plugin(RenderPlugin)
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(exit_game));
    }
}

fn exit_game(
    mut commands: Commands,
    chunks: Query<Entity>,
    keys: Res<Input<KeyCode>>,
    mut state: ResMut<State<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for chunk_e in chunks.iter() {
            commands.entity(chunk_e).despawn();
        }

        commands.remove_resource::<Map>();

        state.set(GameState::MainMenu).unwrap();
    }
}
