use bevy::prelude::*;

use super::{block::Block, render::RenderChunk};

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_AREA: usize = CHUNK_SIZE * CHUNK_SIZE;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[derive(Component)]
pub struct Chunk {
    blocks: Vec<Option<Block>>,
    dirty: bool,
}

impl Chunk {
    pub fn generate(pos: IVec3) -> Self {
        let mut chunk = vec![None; CHUNK_VOLUME];
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    if ((x % 30) as f32 / 30. * CHUNK_SIZE as f32)
                        > ((pos.y * CHUNK_SIZE as i32) + y as i32) as f32
                    {
                        chunk[x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE] = Some(Block {
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

        let mut chunk = Self {
            blocks: chunk,
            dirty: true,
        };

        for x in 1..CHUNK_SIZE - 1 {
            for y in 1..CHUNK_SIZE - 1 {
                for z in 1..CHUNK_SIZE - 1 {
                    if chunk.at((x, y, z - 1)).is_some()
                        && chunk.at((x, y, z + 1)).is_some()
                        && chunk.at((x, y - 1, z)).is_some()
                        && chunk.at((x, y + 1, z)).is_some()
                        && chunk.at((x - 1, y, z)).is_some()
                        && chunk.at((x + 1, y, z)).is_some()
                    {
                        if let Some(block) = chunk.at_mut((x, y, z)) {
                            block.visible = false;
                        }
                    }
                }
            }
        }

        chunk
    }

    fn at(&self, (x, y, z): (usize, usize, usize)) -> Option<&Block> {
        self.blocks
            .get(x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE)
            .and_then(|block| block.as_ref())
    }

    fn at_mut(&mut self, (x, y, z): (usize, usize, usize)) -> Option<&mut Block> {
        self.blocks
            .get_mut(x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE)
            .and_then(|block| block.as_mut())
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
