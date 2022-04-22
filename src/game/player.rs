use bevy::prelude::*;

use super::chunk::CHUNK_SIZE;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(update_chunk_pos);
    }
}

#[derive(Component, Default, Deref, DerefMut)]
pub struct ChunkPos(IVec3);

fn update_chunk_pos(mut chunk_poses: Query<(&mut ChunkPos, &Transform)>) {
    for (mut chunk_pos, tf) in chunk_poses.iter_mut() {
        let new_pos = (tf.translation / CHUNK_SIZE as f32).as_ivec3();

        if new_pos != **chunk_pos {
            **chunk_pos = new_pos;
        }
    }
}
