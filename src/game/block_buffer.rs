use bevy::{
    prelude::*,
    render::{
        render_resource::{Buffer, BufferAddress, BufferDescriptor, BufferUsages},
        renderer::{RenderDevice, RenderQueue},
    },
    utils::HashMap,
};
use bytemuck::cast_slice;

use super::render::GpuBlock;

pub struct BlockBuffer {
    blocks: HashMap<IVec3, Vec<GpuBlock>>,
    buffer: Option<Buffer>,
    capacity: usize,
    block_size: usize,
    buffer_usages: BufferUsages,
}

impl Default for BlockBuffer {
    fn default() -> Self {
        Self {
            blocks: HashMap::default(),
            buffer: None,
            capacity: 0,
            buffer_usages: BufferUsages::all(),
            block_size: std::mem::size_of::<GpuBlock>(),
        }
    }
}

impl BlockBuffer {
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
        self.blocks
            .iter()
            .fold(0, |acc, (_, blocks)| acc + blocks.len())
    }

    pub fn insert(&mut self, pos: IVec3, value: Vec<GpuBlock>) -> usize {
        let index = self.len();
        self.blocks.insert(pos, value);
        index
    }

    pub fn remove(&mut self, pos: IVec3) {
        self.blocks.remove(&pos);
    }

    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = self.block_size * capacity;
            self.buffer = Some(device.create_buffer(&BufferDescriptor {
                label: None,
                size: size as BufferAddress,
                usage: BufferUsages::COPY_DST | self.buffer_usages,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.blocks.is_empty() {
            return;
        }
        self.reserve(self.len(), device);
        if let Some(buffer) = &self.buffer {
            let mut offset = 0;
            for (_, blocks) in &self.blocks {
                let range = 0..self.block_size * blocks.len();
                let bytes: &[u8] = cast_slice(&blocks);
                queue.write_buffer(buffer, offset, &bytes[range]);
                offset += (self.block_size * blocks.len()) as u64;
            }
        }
    }
}
