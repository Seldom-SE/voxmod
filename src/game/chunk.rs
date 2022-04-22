use bevy::prelude::*;
use rand::{prelude::*, thread_rng};

use super::{block::Block, render::RenderChunk};

pub const CHUNK_SIZE: usize = 32;

// TODO Benchmark this vs vec![Entity; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] / Vec::reserve_exact
#[derive(Component, Deref)]
pub struct Chunk([Box<[Box<[Option<Block>; CHUNK_SIZE]>; CHUNK_SIZE]>; CHUNK_SIZE]);

impl Chunk {
    pub fn generate(pos: IVec3) -> Self {
        let mut rng = thread_rng();

        let mut x = 0.;

        Self([(); CHUNK_SIZE].map(|_| {
            let mut y = 0.;

            let slice = [(); CHUNK_SIZE].map(|_| {
                let mut z = 0.;

                let row = [(); CHUNK_SIZE].map(|_| {
                    let block = ((rng.gen::<f32>() * CHUNK_SIZE as f32)
                        > (pos.y * CHUNK_SIZE as i32) as f32 + y)
                        .then(|| Block {
                            color: Color::rgb(rng.gen(), rng.gen(), rng.gen()),
                        });

                    z += 1.;
                    block
                });

                y += 1.;
                Box::new(row)
            });

            x += 1.;
            Box::new(slice)
        }))
    }

    pub fn extract(&self, commands: &mut Commands, chunk_e: Entity, pos: IVec3) {
        commands.get_or_spawn(chunk_e).insert(RenderChunk {
            blocks: Some((**self).clone().map(|slice| {
                Box::new(slice.map(|row| Box::new(row.map(|block| block.map(|block| block.color)))))
            })),
            pos,
        });
    }
}
