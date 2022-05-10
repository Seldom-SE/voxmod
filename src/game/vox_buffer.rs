use std::mem::size_of;

use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferAddress, BufferDescriptor, BufferUsages},
        renderer::{RenderDevice, RenderQueue},
    },
    utils::HashMap,
};
use bytemuck::cast_slice;

use super::render::GpuVox;

pub struct VoxBuffer {
    voxes: HashMap<IVec3, Vec<GpuVox>>,
    buffer: Option<Buffer>,
    capacity: usize,
    vox_size: usize,
    buffer_usages: BufferUsages,
}

impl Default for VoxBuffer {
    fn default() -> Self {
        Self {
            voxes: HashMap::default(),
            buffer: None,
            capacity: 0,
            buffer_usages: BufferUsages::all(),
            vox_size: size_of::<GpuVox>(),
        }
    }
}

impl VoxBuffer {
    pub fn new(buffer_usage: BufferUsages) -> Self {
        Self {
            buffer_usages: buffer_usage,
            ..default()
        }
    }

    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.voxes
            .iter()
            .fold(0, |acc, (_, voxes)| acc + voxes.len())
    }

    pub fn insert(&mut self, pos: IVec3, value: Vec<GpuVox>) -> usize {
        let index = self.len();
        self.voxes.insert(pos, value);
        index
    }

    pub fn remove(&mut self, pos: IVec3) {
        self.voxes.remove(&pos);
    }

    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = self.vox_size * capacity;
            self.buffer = Some(device.create_buffer(&BufferDescriptor {
                label: None,
                size: size as BufferAddress,
                usage: BufferUsages::COPY_DST | self.buffer_usages,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.voxes.is_empty() {
            return;
        }
        self.reserve(self.len(), device);
        if let Some(buffer) = &self.buffer {
            let mut offset = 0;
            for (_, voxes) in &self.voxes {
                let range = 0..self.vox_size * voxes.len();
                let bytes: &[u8] = cast_slice(&voxes);
                queue.write_buffer(buffer, offset, &bytes[range]);
                offset += (self.vox_size * voxes.len()) as u64;
            }
        }
    }
}
