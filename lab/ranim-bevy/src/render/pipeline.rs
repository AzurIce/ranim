use std::borrow::Cow;

use bevy::{
    core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT,
    pbr::{MeshPipeline, MeshPipelineKey},
    prelude::*,
    render::render_resource::{
        BindGroupLayoutDescriptor, BindGroupLayoutEntries, BlendState, ColorTargetState,
        ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, FragmentState,
        MultisampleState, PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor,
        ShaderStages, SpecializedRenderPipeline, StencilFaceState, StencilState, VertexState,
        binding_types::storage_buffer_read_only,
    },
};

use crate::shader::RANIM_VITEM_SHADER_HANDLE;

use super::gpu::{GpuVec4, InstanceInfo, ItemInfo, PlaneData};

#[derive(Resource, Clone)]
pub(crate) struct RanimVItemPipeline {
    pub(crate) shader: Handle<Shader>,
    pub(crate) mesh_pipeline: MeshPipeline,
    pub(crate) items_layout: BindGroupLayoutDescriptor,
}

impl RanimVItemPipeline {
    pub(crate) fn new(mesh_pipeline: MeshPipeline) -> Self {
        Self {
            shader: RANIM_VITEM_SHADER_HANDLE,
            mesh_pipeline,
            items_layout: BindGroupLayoutDescriptor::new(
                "ranim_vitem_items_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        storage_buffer_read_only::<ItemInfo>(false),
                        storage_buffer_read_only::<PlaneData>(false),
                        storage_buffer_read_only::<GpuVec4>(false),
                        storage_buffer_read_only::<GpuVec4>(false),
                        storage_buffer_read_only::<GpuVec4>(false),
                        storage_buffer_read_only::<f32>(false),
                        storage_buffer_read_only::<InstanceInfo>(false),
                    ),
                ),
            ),
        }
    }
}

impl SpecializedRenderPipeline for RanimVItemPipeline {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        if key.contains(MeshPipelineKey::OIT_ENABLED) {
            shader_defs.push("OIT_ENABLED".into());
        }
        if key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }
        if key.msaa_samples() > 1 {
            shader_defs.push("MULTISAMPLED".into());
        }

        let blend = if key.contains(MeshPipelineKey::OIT_ENABLED) {
            None
        } else {
            Some(BlendState::ALPHA_BLENDING)
        };
        let view_layout = self.mesh_pipeline.get_view_layout(key.into());

        RenderPipelineDescriptor {
            label: Some(Cow::Borrowed("ranim_vitem_pipeline")),
            layout: vec![
                view_layout.main_layout,
                view_layout.binding_array_layout,
                self.items_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some(Cow::Borrowed("vs_main")),
                buffers: vec![],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some(Cow::Borrowed("fs_main")),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format(),
                    blend,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            ..default()
        }
    }
}
