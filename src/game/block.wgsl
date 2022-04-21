struct View {
    view_proj: mat4x4<f32>;
    view: mat4x4<f32>;
    view_inv: mat4x4<f32>;
    proj: mat4x4<f32>;
    world_pos: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};

struct Block {
    pos: vec4<f32>;
    color: vec4<f32>;
};

struct Blocks {
    blocks: array<Block>;
};

[[group(0), binding(0)]]
var<uniform> view: View;

[[group(1), binding(0)]]
var<storage> blocks: Blocks;

struct VertOut {
    [[builtin(position)]] clip_pos: vec4<f32>;
    [[location(0)]] world_pos: vec4<f32>;
    [[location(1)]] world_norm: vec3<f32>;
    [[location(2)]] uvw: vec3<f32>;
    [[location(3)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vertex([[builtin(vertex_index)]] vert_i: u32) -> VertOut {
    var out: VertOut;

    let curr_block = blocks.blocks[vert_i >> 3u];

    let local_cam_pos = view.world_pos - curr_block.pos.xyz;
    let vx = vert_i ^ (
        u32(local_cam_pos.y < 0.0) << 2u |
        u32(local_cam_pos.z < 0.0) << 1u |
        u32(local_cam_pos.x < 0.0)
    );

    out.uvw = vec3<f32>(vec3<i32>(
        i32(vx & 0x1u),
        i32((vx & 0x4u) >> 2u),
        i32((vx & 0x2u) >> 1u)
    ));
    out.world_pos = vec4<f32>(curr_block.pos.xyz + out.uvw - 0.5, 1.0);
    out.world_norm = vec3<f32>(0.0, 0.0, 1.0);
    out.clip_pos = view.view_proj * out.world_pos;
    out.color = curr_block.color;

    return out;
}

struct FragIn {
    [[builtin(front_facing)]] is_front: bool;
    [[location(0)]] world_pos: vec4<f32>;
    [[location(1)]] world_norm: vec3<f32>;
    [[location(2)]] uvw: vec3<f32>;
    [[location(3)]] color: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragIn) -> [[location(0)]] vec4<f32> {
    return in.color;
}