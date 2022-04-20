use std::f32::consts::FRAC_PI_4;

use bevy::{prelude::*, render::camera::Camera3d};

use crate::{chunk::Chunk, state::GameState};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(init_game))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(exit_game));
    }
}

fn init_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let chunk = Chunk::generate(&mut commands, &mut meshes, &mut materials);
    commands.insert_resource(chunk);

    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -FRAC_PI_4,
            FRAC_PI_4 * 1.5,
            FRAC_PI_4,
        )),
        ..default()
    });
}

fn exit_game(
    mut commands: Commands,
    cams: Query<Entity, With<Camera3d>>,
    keys: Res<Input<KeyCode>>,
    chunk: Res<Chunk>,
    mut state: ResMut<State<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for slice in chunk.iter() {
            for row in slice.iter() {
                for block in row.iter() {
                    commands.entity(*block).despawn();
                }
            }
        }

        for cam in cams.iter() {
            commands.entity(cam).despawn();
        }

        state.set(GameState::MainMenu).unwrap();
    }
}
