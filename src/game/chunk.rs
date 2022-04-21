use bevy::prelude::*;
use rand::{prelude::*, thread_rng};

use super::{block::BlockColor, render::RenderChunk};

pub const CHUNK_SIZE: usize = 32;

// TODO Benchmark this vs vec![Entity; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] / Vec::reserve_exact
#[derive(Component)]
pub struct Chunk {
    blocks: [Box<[Box<[Entity; CHUNK_SIZE]>; CHUNK_SIZE]>; CHUNK_SIZE],
    pos: IVec3,
    dirty: bool,
}

impl Chunk {
    pub fn spawn(commands: &mut Commands) {
        let mut rng = thread_rng();

        let mut x = 0.;

        let chunk = Chunk {
            blocks: [(); CHUNK_SIZE].map(|_| {
                let mut y = 0.;

                let slice = [(); CHUNK_SIZE].map(|_| {
                    let mut z = 0.;

                    let row = [(); CHUNK_SIZE].map(|_| {
                        let block = if rng.gen() {
                            commands
                                .spawn()
                                .insert(BlockColor(Color::rgb(rng.gen(), rng.gen(), rng.gen())))
                                .id()
                        } else {
                            commands.spawn().id()
                        };

                        z += 1.;
                        block
                    });

                    y += 1.;
                    Box::new(row)
                });

                x += 1.;
                Box::new(slice)
            }),
            pos: IVec3::splat(0),
            dirty: true,
        };

        commands.spawn().insert(chunk);
    }

    pub fn extract(
        &mut self,
        commands: &mut Commands,
        chunk_e: Entity,
        blocks: &Query<&BlockColor>,
    ) {
        if self.dirty {
            commands.get_or_spawn(chunk_e).insert(RenderChunk {
                blocks: Some(self.blocks.clone().map(|slice| {
                    Box::new(
                        slice.map(|row| {
                            Box::new(row.map(|block_e| blocks.get(block_e).ok().cloned()))
                        }),
                    )
                })),
                pos: self.pos,
            });
            self.dirty = false;
        } else {
            commands.get_or_spawn(chunk_e).insert(RenderChunk {
                blocks: None,
                pos: self.pos,
            });
        }
    }

    pub fn despawn(&self, commands: &mut Commands) {
        for slice in self.blocks.iter() {
            for row in slice.iter() {
                for block in row.iter() {
                    commands.entity(*block).despawn();
                }
            }
        }
    }
}
