mod block;
mod cam;
mod chunk;
mod render;

use bevy::{prelude::*, render::camera::Camera3d};

use crate::state::GameState;

use self::{cam::CamPlugin, chunk::Chunk, render::RenderPlugin};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CamPlugin)
            .add_plugin(RenderPlugin)
            .add_system_set(SystemSet::on_enter(GameState::Game).with_system(init_game))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(exit_game));
    }
}

fn init_game(mut commands: Commands) {
    Chunk::spawn(&mut commands);
}

fn exit_game(
    mut commands: Commands,
    cams: Query<Entity, With<Camera3d>>,
    chunks: Query<(Entity, &Chunk)>,
    keys: Res<Input<KeyCode>>,
    mut state: ResMut<State<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for (chunk_e, chunk) in chunks.iter() {
            chunk.despawn(&mut commands);
            commands.entity(chunk_e).despawn();
        }

        for cam in cams.iter() {
            commands.entity(cam).despawn();
        }

        state.set(GameState::MainMenu).unwrap();
    }
}
