use std::f32::consts::PI;

use bevy::{math::const_ivec3, prelude::*, tasks::Task};
use futures_lite::future::{block_on, poll_once};

use crate::state::GameState;

use super::{block::Block, map::RENDER_RADIUS_F32, render::RenderChunk};

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub struct ChunkPlugin;

impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_update(GameState::Game).with_system(resolve_chunks));
    }
}

#[derive(Component)]
pub struct Chunk {
    blocks: Vec<Option<Block>>,
    dirty: bool,
}

static ADJACENTS: &[IVec3] = &[
    const_ivec3!([1, 0, 0]),
    const_ivec3!([-1, 0, 0]),
    const_ivec3!([0, 1, 0]),
    const_ivec3!([0, -1, 0]),
    const_ivec3!([0, 0, 1]),
    const_ivec3!([0, 0, -1]),
];

impl Chunk {
    fn flatten(pos: IVec3) -> usize {
        pos.x as usize + pos.y as usize * CHUNK_SIZE + pos.z as usize * CHUNK_AREA
    }

    pub fn expand(pos: usize) -> IVec3 {
        IVec3::new(
            (pos % CHUNK_SIZE) as i32,
            (pos / CHUNK_SIZE % CHUNK_SIZE) as i32,
            (pos / CHUNK_AREA) as i32,
        )
    }

    pub fn generate(pos: IVec3) -> Self {
        let mut blocks = vec![None; CHUNK_VOLUME];
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    if ((x % 30) as f32 / 30. * CHUNK_SIZE as f32)
                        > ((pos.y * CHUNK_SIZE as i32) + y as i32) as f32
                    {
                        blocks[Self::flatten(IVec3::new(x as i32, y as i32, z as i32))] =
                            Some(Block {
                                color: Color::rgb(
                                    (x % 100) as f32 / 100.,
                                    (y % 10) as f32 / 10.,
                                    (z % 55) as f32 / 55.,
                                ),
                                visible: true,
                            });
                    }
                }
            }
        }

        for x in 1..CHUNK_SIZE as i32 - 1 {
            for y in 1..CHUNK_SIZE as i32 - 1 {
                for z in 1..CHUNK_SIZE as i32 - 1 {
                    if ADJACENTS
                        .iter()
                        .all(|adj| blocks[Chunk::flatten(IVec3::new(x, y, z) + *adj)].is_some())
                    {
                        if let Some(block) = &mut blocks[Chunk::flatten(IVec3::new(x, y, z))] {
                            block.visible = false;
                        }
                    }
                }
            }
        }

        Self {
            blocks,
            dirty: true,
        }
    }

    pub fn extract(&mut self, commands: &mut Commands, chunk_e: Entity, pos: IVec3) {
        if self.dirty {
            commands.get_or_spawn(chunk_e).insert(RenderChunk {
                blocks: self.blocks.clone(),
                pos,
            });
            self.dirty = false;
        }
    }
}

const GEN_RATE_LIMIT: f32 = 0.01;
const PI_4_3: f32 = 4. / 3. * PI;
const GEN_LIMIT: usize =
    (PI_4_3 * RENDER_RADIUS_F32 * RENDER_RADIUS_F32 * RENDER_RADIUS_F32 * GEN_RATE_LIMIT) as usize;

fn resolve_chunks(mut commands: Commands, mut loading_chunks: Query<(Entity, &mut Task<Chunk>)>) {
    let mut gen_count = 0;
    for (chunk_e, mut task) in loading_chunks.iter_mut() {
        if let Some(chunk) = block_on(poll_once(&mut *task)) {
            commands
                .entity(chunk_e)
                .insert(chunk)
                .remove::<Task<Chunk>>();

            gen_count += 1;
            if gen_count >= GEN_LIMIT {
                break;
            }
        }
    }
}
