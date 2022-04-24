use std::mem::take;

use bevy::{
    prelude::*,
    render::camera::Camera3d,
    tasks::AsyncComputeTaskPool,
    utils::{HashMap, HashSet},
};

use crate::state::GameState;

use super::{chunk::Chunk, player::ChunkPos, render::RemovedChunks, DespawnQueue};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(init_map))
            .add_system_set(SystemSet::on_update(GameState::Game).with_system(load_chunks));
    }
}

#[derive(Default)]
pub struct Map {
    chunks: HashMap<IVec3, Entity>,
    removed_chunks: Vec<IVec3>,
}

const RENDER_RADIUS: i32 = 4;
pub const RENDER_RADIUS_F32: f32 = RENDER_RADIUS as f32;

impl Map {
    fn load_chunks(
        &mut self,
        commands: &mut Commands,
        pos: IVec3,
        thread_pool: &AsyncComputeTaskPool,
        despawn_queue: &mut DespawnQueue,
    ) {
        let mut expected_chunks = HashSet::default();
        for x in -RENDER_RADIUS..=RENDER_RADIUS {
            for y in -RENDER_RADIUS..=RENDER_RADIUS {
                for z in -RENDER_RADIUS..=RENDER_RADIUS {
                    if ((x * x + y * y + z * z) as f32) < RENDER_RADIUS_F32 * RENDER_RADIUS_F32 {
                        expected_chunks.insert(pos + IVec3::new(x, y, z));
                    }
                }
            }
        }

        let mut to_remove = Vec::default();
        for (chunk_pos, chunk_e) in self.chunks.iter() {
            if !expected_chunks.contains(chunk_pos) {
                to_remove.push(*chunk_pos);
                expected_chunks.remove(chunk_pos);
                despawn_queue.push(*chunk_e);
            }
        }

        for pos in to_remove {
            self.chunks.remove(&pos);
            self.removed_chunks.push(pos);
        }

        for chunk_pos in expected_chunks {
            if !self.chunks.contains_key(&chunk_pos) {
                self.chunks.insert(
                    chunk_pos,
                    commands
                        .spawn()
                        .insert(thread_pool.spawn(async move { Chunk::generate(chunk_pos) }))
                        .id(),
                );
            }
        }
    }

    pub fn extract(
        &mut self,
        commands: &mut Commands,
        chunks: &mut Query<&mut Chunk>,
        removed_chunks: &mut RemovedChunks,
    ) {
        for (pos, chunk_e) in self.chunks.iter() {
            if let Ok(mut chunk) = chunks.get_mut(*chunk_e) {
                chunk.extract(commands, *chunk_e, *pos);
            }
        }

        **removed_chunks = take(&mut self.removed_chunks);
    }
}

fn init_map(mut commands: Commands) {
    commands.init_resource::<Map>();
}

fn load_chunks(
    mut commands: Commands,
    players: Query<&ChunkPos, (With<Camera3d>, Changed<ChunkPos>)>,
    thread_pool: Res<AsyncComputeTaskPool>,
    mut map: ResMut<Map>,
    mut despawn_queue: ResMut<DespawnQueue>,
) {
    for pos in players.iter() {
        map.load_chunks(&mut commands, **pos, &thread_pool, &mut despawn_queue);
    }
}
