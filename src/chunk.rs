use bevy::prelude::*;
use rand::{prelude::*, thread_rng};

const CHUNK_SIZE: usize = 32;

// TODO Maybe use https://github.com/superdump/bevy-vertex-pulling
// TODO Benchmark this vs vec![Entity; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE] / Vec::reserve_exact
#[derive(Deref)]
pub struct Chunk([Box<[Box<[Entity; CHUNK_SIZE]>; CHUNK_SIZE]>; CHUNK_SIZE]);

impl Chunk {
    pub fn generate(
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> Chunk {
        let mut rng = thread_rng();

        let cube = meshes.add(Mesh::from(shape::Cube { size: 1. }));
        let colors =
            [(); 16].map(|_| materials.add(Color::rgb(rng.gen(), rng.gen(), rng.gen()).into()));

        let mut x = 0.;

        Chunk([(); CHUNK_SIZE].map(|_| {
            let mut y = 0.;

            let slice = [(); CHUNK_SIZE].map(|_| {
                let mut z = 0.;

                let row = [(); CHUNK_SIZE].map(|_| {
                    let block = if rng.gen() {
                        commands
                            .spawn_bundle(PbrBundle {
                                mesh: cube.clone(),
                                material: colors.choose(&mut rng).unwrap().clone(),
                                transform: Transform::from_xyz(x, y, z),
                                ..default()
                            })
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
        }))
    }
}
