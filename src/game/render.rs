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
            BufferBindingType, BufferInitDescriptor, BufferSize, BufferUsages, BufferVec,
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

use super::{
    block::BlockColor,
    chunk::{Chunk, CHUNK_SIZE},
};

pub struct RenderPlugin;

const BLOCKS_PASS: &str = "blocks_pass";

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.world.resource_mut::<Assets<Shader>>().set_untracked(
            BLOCKS_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("block.wgsl")),
        );

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<DrawFunctions<BlocksPhaseItem>>()
            .add_render_command::<BlocksPhaseItem, DrawBlocks>()
            .init_resource::<BlocksPipeline>()
            .init_resource::<GpuBlocks>()
            .add_system_to_stage(RenderStage::Extract, extract_blocks_phase)
            .add_system_to_stage(RenderStage::Extract, extract_blocks)
            .add_system_to_stage(RenderStage::Prepare, prepare_blocks)
            .add_system_to_stage(RenderStage::Queue, queue_blocks);

        let blocks_pass_node = BlocksPassNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();

        let draw_3d_graph = graph.get_sub_graph_mut(NAME).unwrap();
        draw_3d_graph.add_node(BLOCKS_PASS, blocks_pass_node);
        draw_3d_graph.add_node_edge(BLOCKS_PASS, MAIN_PASS).unwrap();
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                VIEW_ENTITY,
                BLOCKS_PASS,
                BlocksPassNode::IN_VIEW,
            )
            .unwrap();
    }
}

struct BlocksPhaseItem {
    e: Entity,
    draw_fn: DrawFunctionId,
}

impl PhaseItem for BlocksPhaseItem {
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

impl EntityPhaseItem for BlocksPhaseItem {
    #[inline]
    fn entity(&self) -> Entity {
        self.e
    }
}

struct SetBlocksPipeline;

impl<P: PhaseItem> RenderCommand<P> for SetBlocksPipeline {
    type Param = (SRes<PipelineCache>, SRes<BlocksPipeline>);

    #[inline]
    fn render<'w>(
        _: Entity,
        _: &P,
        params: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (pipeline_cache, blocks_pipeline) = params;

        if let Some(pipeline) = pipeline_cache
            .into_inner()
            .get_render_pipeline(blocks_pipeline.id)
        {
            pass.set_render_pipeline(pipeline);
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

#[derive(Component, Deref)]
struct GpuBlocksBindGroup(BindGroup);

struct SetGpuBlocksBindGroup<const I: usize>;

impl<const I: usize> EntityRenderCommand for SetGpuBlocksBindGroup<I> {
    type Param = SQuery<Read<GpuBlocksBindGroup>>;

    #[inline]
    fn render<'w>(
        _: Entity,
        item: Entity,
        gpu_blocks_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &gpu_blocks_bind_groups.get_inner(item).unwrap(), &[]);

        RenderCommandResult::Success
    }
}

struct DrawVertexPulledBlocks;

impl EntityRenderCommand for DrawVertexPulledBlocks {
    type Param = SRes<GpuBlocks>;

    #[inline]
    fn render<'w>(
        _: Entity,
        _: Entity,
        gpu_blocks: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let gpu_blocks = gpu_blocks.into_inner();
        pass.set_index_buffer(
            gpu_blocks.i_buffer.as_ref().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.draw_indexed(0..gpu_blocks.i_count as u32, 0, 0..1);

        RenderCommandResult::Success
    }
}

type DrawBlocks = (
    SetBlocksPipeline,
    SetShadowViewBindGroup<0>,
    SetGpuBlocksBindGroup<1>,
    DrawVertexPulledBlocks,
);

struct BlocksPipeline {
    id: CachedRenderPipelineId,
    layout: BindGroupLayout,
}

const BLOCKS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2119073280875309064);

impl FromWorld for BlocksPipeline {
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

        let blocks_layout =
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
                    label: Some("cubes_pipeline".into()),
                    layout: Some(vec![view_layout, blocks_layout.clone()]),
                    vertex: VertexState {
                        shader: BLOCKS_SHADER_HANDLE.typed(),
                        shader_defs: vec![],
                        entry_point: "vertex".into(),
                        buffers: vec![],
                    },
                    fragment: Some(FragmentState {
                        shader: BLOCKS_SHADER_HANDLE.typed(),
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
            layout: blocks_layout,
        }
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct GpuBlock {
    pos: Vec4,
    color: [f32; 4],
}

struct GpuBlocks {
    i_buffer: Option<Buffer>,
    i_count: u32,
    insts: BufferVec<GpuBlock>,
}

impl Default for GpuBlocks {
    fn default() -> Self {
        Self {
            i_buffer: None,
            i_count: 0,
            insts: BufferVec::<GpuBlock>::new(BufferUsages::STORAGE),
        }
    }
}

#[derive(Component)]
pub struct RenderChunk {
    pub blocks: Option<[Box<[Box<[Option<BlockColor>; CHUNK_SIZE]>; CHUNK_SIZE]>; CHUNK_SIZE]>,
    pub pos: IVec3,
}

impl RenderChunk {
    fn prepare(
        &self,
        render_device: &RenderDevice,
        render_queue: &RenderQueue,
        gpu_blocks: &mut GpuBlocks,
    ) {
        if let Some(blocks) = &self.blocks {
            let block_pos = self.pos * CHUNK_SIZE as i32;

            for (x, slice) in blocks.iter().enumerate() {
                for (y, row) in slice.iter().enumerate() {
                    for (z, block) in row.iter().enumerate() {
                        if let Some(block) = block {
                            gpu_blocks.insts.push(GpuBlock {
                                pos: (IVec3::new(x as i32, y as i32, z as i32) + block_pos)
                                    .as_vec3()
                                    .extend(1.),
                                color: block.as_rgba_f32(),
                            });
                        }
                    }
                }
            }

            gpu_blocks.i_count = gpu_blocks.insts.len() as u32 * BLOCK_I_COUNT as u32;
            gpu_blocks.i_buffer = Some(render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: Some("gpu_blocks_i_buffer"),
                    contents: cast_slice(&gen_i_buffer_data(gpu_blocks.insts.len())),
                    usage: BufferUsages::INDEX,
                },
            ));

            gpu_blocks.insts.write_buffer(render_device, render_queue);
        }
    }
}

struct BlocksPassNode {
    query: QueryState<
        (
            &'static RenderPhase<BlocksPhaseItem>,
            &'static ViewTarget,
            &'static ViewDepthTexture,
        ),
        With<ExtractedView>,
    >,
}

impl BlocksPassNode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for BlocksPassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(BlocksPassNode::IN_VIEW, SlotType::Entity)]
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
        let (cubes_phase, target, depth) = match self.query.get_manual(world, view_e) {
            Ok(query) => query,
            Err(_) => return Ok(()),
        };

        let render_pass = render_ctx
            .command_encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("main_blocks_pass"),
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

        let mut draw_fns = world.resource::<DrawFunctions<BlocksPhaseItem>>().write();
        let mut tracked_pass = TrackedRenderPass::new(render_pass);
        for item in &cubes_phase.items {
            draw_fns
                .get_mut(item.draw_fn)
                .unwrap()
                .draw(world, &mut tracked_pass, view_e, item);
        }

        Ok(())
    }
}

fn extract_blocks_phase(mut commands: Commands, cams: Query<Entity, With<Camera3d>>) {
    if let Ok(cam_e) = cams.get_single() {
        commands
            .get_or_spawn(cam_e)
            .insert(RenderPhase::<BlocksPhaseItem>::default());
    }
}

fn extract_blocks(
    mut commands: Commands,
    mut chunks: Query<(Entity, &mut Chunk)>,
    blocks: Query<&BlockColor>,
) {
    for (chunk_e, mut chunk) in chunks.iter_mut() {
        chunk.extract(&mut commands, chunk_e, &blocks);
    }
}

const BLOCK_BACKFACE_OPT: bool = false;
const BLOCK_I_COUNT: usize = if BLOCK_BACKFACE_OPT {
    3 * 3 * 2
} else {
    3 * 6 * 2
};
const BLOCK_VERTEX_COUNT: usize = 8;

fn gen_i_buffer_data(num_blocks: usize) -> Vec<u32> {
    #[rustfmt::skip]
    let block_is = [
        0u32, 2, 1, 2, 3, 1,
        5, 4, 1, 1, 4, 0,
        0, 4, 6, 0, 6, 2,
        6, 5, 7, 6, 4, 5,
        2, 6, 3, 6, 7, 3,
        7, 1, 3, 7, 5, 1,
    ];

    let num_is = num_blocks * BLOCK_I_COUNT;

    (0..num_is)
        .map(|i| {
            (i / BLOCK_I_COUNT) as u32 * BLOCK_VERTEX_COUNT as u32 + block_is[i % BLOCK_I_COUNT]
        })
        .collect()
}

fn prepare_blocks(
    chunks: Query<&RenderChunk>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut gpu_blocks: ResMut<GpuBlocks>,
) {
    for chunk in chunks.iter() {
        chunk.prepare(&render_device, &render_queue, &mut gpu_blocks);
    }
}

fn queue_blocks(
    mut commands: Commands,
    chunks: Query<Entity, With<RenderChunk>>,
    mut views: Query<&mut RenderPhase<BlocksPhaseItem>>,
    opaque_3d_draw_fns: Res<DrawFunctions<BlocksPhaseItem>>,
    blocks_pipeline: Res<BlocksPipeline>,
    render_device: Res<RenderDevice>,
    gpu_blocks: Res<GpuBlocks>,
) {
    let draw_blocks = opaque_3d_draw_fns.read().get_id::<DrawBlocks>().unwrap();

    for mut opaque_phase in views.iter_mut() {
        for chunk_e in chunks.iter() {
            commands.get_or_spawn(chunk_e).insert(GpuBlocksBindGroup(
                render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("gpu_blocks_bind_group"),
                    layout: &blocks_pipeline.layout,
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: gpu_blocks.insts.buffer().unwrap().as_entire_binding(),
                    }],
                }),
            ));

            opaque_phase.add(BlocksPhaseItem {
                e: chunk_e,
                draw_fn: draw_blocks,
            });
        }
    }
}