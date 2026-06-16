// Vertex shader
struct CameraUniform {
    vp: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos : vec3<f32>,
};

// Flat array of verticies
const vertex_positions = array<f32, 12>(-1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0);
const grid_scale = 100.0;

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = vertex_positions[in_vertex_index * 2] * grid_scale;
    let z = vertex_positions[in_vertex_index * 2 + 1] * grid_scale;
    out.world_pos = vec3<f32>(x, 0.0, z);
    out.clip_position = camera.vp * vec4<f32>(x, 0.0, z, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let minor =
        min(
            abs(fract(in.world_pos.x / 0.1)),
            abs(fract(in.world_pos.z / 0.1))
        );

    let major =
        min(
            abs(fract(in.world_pos.x / 1.0)),
            abs(fract(in.world_pos.z / 1.0))
        );

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    if major < 0.01 {
        color = vec4<f32>(0.4, 0.4, 0.4, 1.0);
    } else if minor < 0.01 {
        color = vec4<f32>(0.2, 0.2, 0.2, 1.0);
    }

    if abs(in.world_pos.x) < 0.02 {
        color = vec4<f32>(0.0, 0.5, 1.0, 1.0);
    }

    if abs(in.world_pos.z) < 0.02 {
        color = vec4<f32>(1.0, 0.2, 0.2, 1.0);
    }

    return color;
}
