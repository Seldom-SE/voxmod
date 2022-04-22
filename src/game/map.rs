use bevy::{
    prelude::*,
    render::camera::Camera3d,
    utils::{HashMap, HashSet},
};

use crate::state::GameState;

use super::{chunk::Chunk, player::ChunkPos};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(init_map))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(load_chunks));
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct Map(HashMap<IVec3, Entity>);

impl Map {
    pub fn extract(&self, commands: &mut Commands, chunks: &Query<&Chunk>) {
        for (pos, chunk_e) in self.iter() {
            chunks
                .get(*chunk_e)
                .unwrap()
                .extract(commands, *chunk_e, *pos);
        }
    }
}

fn init_map(mut commands: Commands) {
    commands.init_resource::<Map>();
}

const RENDER_RADIUS: i32 = 1;

fn load_chunks(
    mut commands: Commands,
    players: Query<&ChunkPos, (With<Camera3d>, Changed<ChunkPos>)>,
    mut map: ResMut<Map>,
) {
    for pos in players.iter() {
        let mut expected_chunks = HashSet::default();
        for x in -RENDER_RADIUS..=RENDER_RADIUS {
            for y in -RENDER_RADIUS..=RENDER_RADIUS {
                for z in -RENDER_RADIUS..=RENDER_RADIUS {
                    expected_chunks.insert(**pos + IVec3::new(x, y, z));
                }
            }
        }

        let mut to_remove = Vec::default();
        for (chunk_pos, chunk_e) in map.iter() {
            if !expected_chunks.contains(chunk_pos) {
                to_remove.push(*chunk_pos);
                expected_chunks.remove(chunk_pos);
                commands.entity(*chunk_e).despawn();
            }
        }

        for chunk_pos in to_remove {
            map.remove(&chunk_pos);
        }

        for chunk_pos in expected_chunks {
            if !map.contains_key(&chunk_pos) {
                map.insert(
                    chunk_pos,
                    commands.spawn().insert(Chunk::generate(chunk_pos)).id(),
                );
            }
        }
    }
}
