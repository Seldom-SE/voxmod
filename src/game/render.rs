// Based on https://github.com/superdump/bevy-vertex-pulling

use bevy::{
    core_pipeline::draw_3d_graph::{input::VIEW_ENTITY, node::MAIN_PASS, NAME},
    ecs::system::{
        lifetimeless::{Read, SQuery, SRes},
        SystemParamItem,
    },
    pbr::SetShadowViewBindGroup,
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::Camera3d,
        mesh::PrimitiveTopology,
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
        render_phase::{
            AddRenderCommand, DrawFunctionId, DrawFunctions, EntityPhaseItem, EntityRenderCommand,
            PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, TrackedRenderPass,
        },
        render_resource::{
            std140::AsStd140, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BlendState, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
            DepthStencilState, FragmentState, FrontFace, IndexFormat, LoadOp, MultisampleState,
            Operations, PipelineCache, PolygonMode, PrimitiveState,
            RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
            ShaderStages, StencilFaceState, StencilState, TextureFormat, VertexState,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, ViewDepthTexture, ViewTarget, ViewUniform},
        RenderApp, RenderStage,
    },
};
use bytemuck::{cast_slice, Pod, Zeroable};

use crate::state::GameState;

use super::{
    chunk::{Chunk, CHUNK_SIZE},
    map::Map,
    vox::Vox,
    vox_buffer::VoxBuffer,
};

pub struct RenderPlugin;

const VOXES_PASS: &str = "voxes_pass";

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::MainMenu).with_system(init_bind_group))
            .world
            .resource_mut::<Assets<Shader>>()
            .set_untracked(
                VOXES_SHADER_HANDLE,
                Shader::from_wgsl(include_str!("vox.wgsl")),
            );

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<DrawFunctions<VoxesPhaseItem>>()
            .add_render_command::<VoxesPhaseItem, DrawVoxes>()
            .init_resource::<VoxesPipeline>()
            .init_resource::<GpuVoxes>()
            .init_resource::<RemovedChunks>()
            .add_system_to_stage(RenderStage::Extract, extract_voxes_phase)
            .add_system_to_stage(RenderStage::Extract, extract_voxes)
            .add_system_to_stage(RenderStage::Prepare, prepare_voxes)
            .add_system_to_stage(RenderStage::Queue, queue_voxes);

        let voxes_pass_node = VoxesPassNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();

        let draw_3d_graph = graph.get_sub_graph_mut(NAME).unwrap();
        draw_3d_graph.add_node(VOXES_PASS, voxes_pass_node);
        draw_3d_graph.add_node_edge(VOXES_PASS, MAIN_PASS).unwrap();
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                VIEW_ENTITY,
                VOXES_PASS,
                VoxesPassNode::IN_VIEW,
            )
            .unwrap();
    }
}

struct VoxesPhaseItem {
    e: Entity,
    draw_fn: DrawFunctionId,
}

impl PhaseItem for VoxesPhaseItem {
    type SortKey = u32;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        0
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_fn
    }
}

impl EntityPhaseItem for VoxesPhaseItem {
    #[inline]
    fn entity(&self) -> Entity {
        self.e
    }
}

struct SetVoxesPipeline;

impl<P: PhaseItem> RenderCommand<P> for SetVoxesPipeline {
    type Param = (SRes<PipelineCache>, SRes<VoxesPipeline>);

    #[inline]
    fn render<'w>(
        _: Entity,
        _: &P,
        params: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (pipeline_cache, voxes_pipeline) = params;

        if let Some(pipeline) = pipeline_cache
            .into_inner()
            .get_render_pipeline(voxes_pipeline.id)
        {
            pass.set_render_pipeline(pipeline);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

#[derive(Component, Deref)]
struct GpuVoxesBindGroup(BindGroup);

#[derive(Component)]
struct BindGroupMarker;

struct SetGpuVoxesBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetGpuVoxesBindGroup<I> {
    type Param = SQuery<Read<GpuVoxesBindGroup>>;

    #[inline]
    fn render<'w>(
        _: Entity,
        item: Entity,
        gpu_voxes_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &gpu_voxes_bind_groups.get_inner(item).unwrap(), &[]);

        RenderCommandResult::Success
    }
}

struct DrawVertexPulledVoxes;

impl EntityRenderCommand for DrawVertexPulledVoxes {
    type Param = SRes<GpuVoxes>;

    #[inline]
    fn render<'w>(
        _: Entity,
        _: Entity,
        gpu_voxes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let gpu_voxes = gpu_voxes.into_inner();
        pass.set_index_buffer(
            gpu_voxes.i_buffer.as_ref().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.draw_indexed(0..gpu_voxes.i_count as u32, 0, 0..1);

        RenderCommandResult::Success
    }
}

type DrawVoxes = (
    SetVoxesPipeline,
    SetShadowViewBindGroup<0>,
    SetGpuVoxesBindGroup<1>,
    DrawVertexPulledVoxes,
);

struct VoxesPipeline {
    id: CachedRenderPipelineId,
    layout: BindGroupLayout,
}

const VOXES_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2119073280875309064);

impl FromWorld for VoxesPipeline {
    fn from_world(world: &mut World) -> Self {
        let view_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: BufferSize::new(
                                ViewUniform::std140_size_static() as u64
                            ),
                        },
                        count: None,
                    }],
                    label: Some("shadow_view_layout"),
                });

        let voxes_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: BufferSize::new(0),
                        },
                        count: None,
                    }],
                });

        let pipeline_id =
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("voxes_pipeline".into()),
                    layout: Some(vec![view_layout, voxes_layout.clone()]),
                    vertex: VertexState {
                        shader: VOXES_SHADER_HANDLE.typed(),
                        shader_defs: vec![],
                        entry_point: "vertex".into(),
                        buffers: vec![],
                    },
                    fragment: Some(FragmentState {
                        shader: VOXES_SHADER_HANDLE.typed(),
                        shader_defs: vec![],
                        entry_point: "fragment".into(),
                        targets: vec![ColorTargetState {
                            format: TextureFormat::bevy_default(),
                            blend: Some(BlendState::REPLACE),
                            write_mask: ColorWrites::ALL,
                        }],
                    }),
                    primitive: PrimitiveState {
                        front_face: FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: PolygonMode::Fill,
                        conservative: false,
                        topology: PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                    },
                    depth_stencil: Some(DepthStencilState {
                        format: TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::Greater,
                        stencil: StencilState {
                            front: StencilFaceState::IGNORE,
                            back: StencilFaceState::IGNORE,
                            read_mask: 0,
                            write_mask: 0,
                        },
                        bias: DepthBiasState {
                            constant: 0,
                            slope_scale: 0.,
                            clamp: 0.,
                        },
                    }),
                    multisample: MultisampleState {
                        count: Msaa::default().samples,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                });

        Self {
            id: pipeline_id,
            layout: voxes_layout,
        }
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuVox {
    pos: Vec4,
    color: [f32; 4],
}

struct GpuVoxes {
    i_buffer: Option<Buffer>,
    i_count: u32,
    insts: VoxBuffer,
}

impl Default for GpuVoxes {
    fn default() -> Self {
        Self {
            i_buffer: None,
            i_count: 0,
            insts: VoxBuffer::new(BufferUsages::STORAGE),
        }
    }
}

#[derive(Component)]
pub struct RenderChunk {
    pub voxes: Vec<Option<Vox>>,
    pub pos: IVec3,
}

impl RenderChunk {
    fn prepare(&self, gpu_voxes: &mut GpuVoxes) {
        let vox_pos = self.pos * CHUNK_SIZE as i32;

        gpu_voxes.insts.insert(
            self.pos,
            self.voxes
                .iter()
                .enumerate()
                .filter_map(|(i, vox)| {
                    vox.as_ref().and_then(|vox| {
                        vox.visible.then(|| GpuVox {
                            pos: (vox_pos + (Chunk::expand(i))).as_vec3().extend(1.),
                            color: vox.color.as_rgba_f32(),
                        })
                    })
                })
                .collect(),
        );
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct RemovedChunks(Vec<IVec3>);

struct VoxesPassNode {
    query: QueryState<
        (
            &'static RenderPhase<VoxesPhaseItem>,
            &'static ViewTarget,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl VoxesPassNode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for VoxesPassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(VoxesPassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_ctx: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_e = graph.get_input_entity(Self::IN_VIEW)?;
        let (voxes_phase, target, depth) = match self.query.get_manual(world, view_e) {
            Ok(query) => query,
            Err(_) => return Ok(()),
        };

        let render_pass = render_ctx
            .command_encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("main_voxes_pass"),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

        let mut draw_fns = world.resource::<DrawFunctions<VoxesPhaseItem>>().write();
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in &voxes_phase.items {
            draw_fns
                .get_mut(item.draw_fn)
                .unwrap()
                .draw(world, &mut tracked_pass, view_e, item);
        }

        Ok(())
    }
}

fn init_bind_group(mut commands: Commands) {
    commands.spawn().insert(BindGroupMarker);
}

fn extract_voxes_phase(
    mut commands: Commands,
    cams: Query<Entity, With<Camera3d>>,
    bind_groups: Query<Entity, With<BindGroupMarker>>,
) {
    for bind_group_e in bind_groups.iter() {
        commands.get_or_spawn(bind_group_e).insert(BindGroupMarker);
    }

    if let Ok(cam_e) = cams.get_single() {
        commands
            .get_or_spawn(cam_e)
            .insert(RenderPhase::<VoxesPhaseItem>::default());
    }
}

fn extract_voxes(mut commands: Commands, mut chunks: Query<&mut Chunk>, map: Option<ResMut<Map>>) {
    let mut removed_chunks = default();

    if let Some(mut map) = map {
        map.extract(&mut commands, &mut chunks, &mut removed_chunks);
    }

    commands.insert_resource(removed_chunks);
}

const VOX_BACKFACE_OPT: bool = true;
const VOX_I_COUNT: usize = if VOX_BACKFACE_OPT {
    3 * 3 * 2
} else {
    3 * 6 * 2
};
const VOX_VERTEX_COUNT: usize = 8;

fn gen_i_buffer_data(vox_count: usize) -> Vec<u32> {
    #[rustfmt::skip]
    let vox_is = [
        0, 2, 1, 2, 3, 1,
        5, 4, 1, 1, 4, 0,
        0, 4, 6, 0, 6, 2,
        6, 5, 7, 6, 4, 5,
        2, 6, 3, 6, 7, 3,
        7, 1, 3, 7, 5, 1,
    ];

    let i_count = vox_count * VOX_I_COUNT;

    (0..i_count)
        .map(|i| (i / VOX_I_COUNT) as u32 * VOX_VERTEX_COUNT as u32 + vox_is[i % VOX_I_COUNT])
        .collect()
}

fn prepare_voxes(
    chunks: Query<&RenderChunk>,
    removed_chunks: Res<RemovedChunks>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut gpu_voxes: ResMut<GpuVoxes>,
) {
    if !chunks.is_empty() || !removed_chunks.is_empty() {
        for removed_chunk in removed_chunks.iter() {
            gpu_voxes.insts.remove(*removed_chunk);
        }

        for chunk in chunks.iter() {
            chunk.prepare(&mut gpu_voxes);
        }

        gpu_voxes.i_count = gpu_voxes.insts.len() as u32 * VOX_I_COUNT as u32;

        gpu_voxes.i_buffer = Some(
            render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("gpu_voxes_i_buffer"),
                contents: cast_slice(&gen_i_buffer_data(gpu_voxes.insts.len())),
                usage: BufferUsages::INDEX,
            }),
        );

        gpu_voxes
            .insts
            .write_buffer(&*render_device, &*render_queue);
    }
}

fn queue_voxes(
    mut commands: Commands,
    bind_groups: Query<Entity, With<BindGroupMarker>>,
    mut views: Query<&mut RenderPhase<VoxesPhaseItem>>,
    opaque_3d_draw_fns: Res<DrawFunctions<VoxesPhaseItem>>,
    voxes_pipeline: Res<VoxesPipeline>,
    render_device: Res<RenderDevice>,
    gpu_voxes: Res<GpuVoxes>,
) {
    if gpu_voxes.insts.buffer().is_none() {
        return;
    }

    let draw_voxes = opaque_3d_draw_fns.read().get_id::<DrawVoxes>().unwrap();

    for mut opaque_phase in views.iter_mut() {
        for bind_group_e in bind_groups.iter() {
            commands
                .get_or_spawn(bind_group_e)
                .insert(GpuVoxesBindGroup(render_device.create_bind_group(
                    &BindGroupDescriptor {
                        label: Some("gpu_voxes_bind_group"),
                        layout: &voxes_pipeline.layout,
                        entries: &[BindGroupEntry {
                            binding: 0,
                            resource: gpu_voxes.insts.buffer().unwrap().as_entire_binding(),
                        }],
                    },
                )));

            opaque_phase.add(VoxesPhaseItem {
                e: bind_group_e,
                draw_fn: draw_voxes,
            });
        }
    }
}
