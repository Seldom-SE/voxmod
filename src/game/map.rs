use bevy::{
    prelude::*,
    render::camera::Camera3d,
    utils::{HashMap, HashSet},
};

use crate::state::GameState;

use super::{chunk::Chunk, player::ChunkPos, render::UnrenderChunk};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(init_map))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(load_chunks));
    }
}

struct RemovedChunk {
    pos: IVec3,
    e: Entity,
}

#[derive(Default)]
pub struct Map {
    chunks: HashMap<IVec3, Entity>,
    removed: Vec<RemovedChunk>,
}

impl Map {
    pub fn extract(&mut self, commands: &mut Commands, chunks: &mut Query<&mut Chunk>) {
        for (pos, chunk_e) in self.chunks.iter() {
            chunks
                .get_mut(*chunk_e)
                .unwrap()
                .extract(commands, *chunk_e, *pos);
        }

        for chunk in self.removed.iter() {
            commands
                .get_or_spawn(chunk.e)
                .insert(UnrenderChunk(chunk.pos));
        }

        self.removed.clear();
    }
}

fn init_map(mut commands: Commands) {
    commands.init_resource::<Map>();
}

const RENDER_RADIUS: i32 = 4;

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
        for (chunk_pos, chunk_e) in map.chunks.iter() {
            if !expected_chunks.contains(chunk_pos) {
                to_remove.push(RemovedChunk {
                    pos: *chunk_pos,
                    e: *chunk_e,
                });
                expected_chunks.remove(chunk_pos);
                commands.entity(*chunk_e).despawn();
            }
        }

        for chunk in to_remove {
            map.chunks.remove(&chunk.pos);
            map.removed.push(chunk);
        }

        for chunk_pos in expected_chunks {
            if !map.chunks.contains_key(&chunk_pos) {
                map.chunks.insert(
                    chunk_pos,
                    commands.spawn().insert(Chunk::generate(chunk_pos)).id(),
                );
            }
        }
    }
}
