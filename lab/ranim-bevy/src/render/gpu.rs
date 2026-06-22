use bevy::{
    prelude::*,
    render::{
        render_resource::{BindGroup, Buffer, BufferInitDescriptor, BufferUsages, ShaderType},
        renderer::{RenderDevice, RenderQueue},
        sync_world::MainEntityHashMap,
    },
};
use bytemuck::{Pod, Zeroable};
use ranim_core::glam::Vec3 as RanimVec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
pub(crate) struct ItemInfo {
    pub(crate) point_offset: u32,
    pub(crate) point_count: u32,
    pub(crate) attr_offset: u32,
    pub(crate) attr_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
pub(crate) struct PlaneData {
    pub(crate) normal: GpuVec4,
    pub(crate) origin: GpuVec4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
pub(crate) struct GpuVec4 {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) w: f32,
}

impl From<Vec4> for GpuVec4 {
    fn from(value: Vec4) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w: value.w,
        }
    }
}

impl From<(RanimVec3, f32)> for GpuVec4 {
    fn from((value, w): (RanimVec3, f32)) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
            w,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, ShaderType)]
pub(crate) struct InstanceInfo {
    pub(crate) world_from_local: Mat4,
    pub(crate) item_index: u32,
    pub(crate) _padding: [u32; 3],
}

pub(crate) struct BufferSlot<T> {
    values: Vec<T>,
    buffer: Option<Buffer>,
}

impl<T> Default for BufferSlot<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            buffer: None,
        }
    }
}

impl<T: Pod> BufferSlot<T> {
    pub(crate) fn set(
        &mut self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        label: &'static str,
        usage: BufferUsages,
        values: Vec<T>,
    ) {
        self.values = if values.is_empty() {
            vec![T::zeroed()]
        } else {
            values
        };

        let data = bytemuck::cast_slice(&self.values);
        let needs_recreate = self
            .buffer
            .as_ref()
            .is_none_or(|buffer| buffer.size() < data.len() as u64);

        if needs_recreate {
            self.buffer = Some(render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage,
            }));
        } else if let Some(buffer) = &self.buffer {
            render_queue.write_buffer(buffer, 0, data);
        }
    }

    pub(crate) fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }
}

#[derive(Resource, Default)]
pub(crate) struct RanimVItemGpuBuffers {
    pub(crate) item_infos: BufferSlot<ItemInfo>,
    pub(crate) planes: BufferSlot<PlaneData>,
    pub(crate) points: BufferSlot<GpuVec4>,
    pub(crate) fill_rgbas: BufferSlot<GpuVec4>,
    pub(crate) stroke_rgbas: BufferSlot<GpuVec4>,
    pub(crate) stroke_widths: BufferSlot<f32>,
    pub(crate) instances: BufferSlot<InstanceInfo>,
    pub(crate) entity_instance_indices: MainEntityHashMap<u32>,
    pub(crate) bind_group: Option<BindGroup>,
    pub(crate) item_count: u32,
}
