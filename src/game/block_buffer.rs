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
    values: HashMap<IVec3, Vec<GpuBlock>>,
    buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
    buffer_usage: BufferUsages,
}

impl Default for BlockBuffer {
    fn default() -> Self {
        Self {
            values: HashMap::default(),
            buffer: None,
            capacity: 0,
            buffer_usage: BufferUsages::all(),
            item_size: std::mem::size_of::<GpuBlock>(),
        }
    }
}

impl BlockBuffer {
    pub fn new(buffer_usage: BufferUsages) -> Self {
        Self {
            buffer_usage,
            ..default()
        }
    }

    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.values
            .iter()
            .fold(0, |acc, (_, blocks)| acc + blocks.len())
    }

    pub fn push(&mut self, pos: IVec3, value: Vec<GpuBlock>) -> usize {
        let index = self.len();
        self.values.insert(pos, value);
        index
    }

    pub fn remove(&mut self, pos: IVec3) {
        self.values.remove(&pos);
    }

    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = self.item_size * capacity;
            self.buffer = Some(device.create_buffer(&BufferDescriptor {
                label: None,
                size: size as BufferAddress,
                usage: BufferUsages::COPY_DST | self.buffer_usage,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.values.is_empty() {
            return;
        }
        self.reserve(self.len(), device);
        if let Some(buffer) = &self.buffer {
            let mut offset = 0;
            for (_, blocks) in &self.values {
                let range = 0..self.item_size * blocks.len();
                let bytes: &[u8] = cast_slice(&blocks);
                queue.write_buffer(buffer, offset, &bytes[range]);
                offset += (self.item_size * blocks.len()) as u64;
            }
        }
    }
}
