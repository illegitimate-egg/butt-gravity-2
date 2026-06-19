// Vertex shader
struct CameraUniform {
    v: mat4x4<f32>,
    vp: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos : vec3<f32>,
    @location(1) view_pos: vec3<f32>,
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

    let world = vec4<f32>(x, 0.0, z, 1.0);
    out.world_pos = world.xyz;
    out.clip_position = camera.vp * world;
    
    out.view_pos = (camera.v * world).xyz;    
    
    return out;
}

// Fragment shader

fn grid_distance(coord: f32, spacing: f32) -> f32 {
    let g = coord / spacing;

    // Distance to nearest grid line in world units
    return abs(fract(g + 0.5) - 0.5) * spacing;
}

fn line_alpha(distance: f32, half_width: f32) -> f32 {
    let aa = fwidth(distance);

    return 1.0 - smoothstep(
        half_width - aa,
        half_width + aa,
        distance
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth = abs(in.view_pos.z);
    let fade = 1.0 - smoothstep(2.0, 20.0, depth);

    let minor_dist =
        min(
            grid_distance(in.world_pos.x, 0.1),
            grid_distance(in.world_pos.z, 0.1)
        );

    let major_dist =
        min(
            grid_distance(in.world_pos.x, 1.0),
            grid_distance(in.world_pos.z, 1.0)
        );

    let minor_alpha = line_alpha(minor_dist, 0.002);
    let major_alpha = line_alpha(major_dist, 0.006);

    var colour = vec3<f32>(0.0);

    colour += vec3<f32>(0.20) * minor_alpha;

    colour = mix(
        colour,
        vec3<f32>(0.40),
        major_alpha
    );

    let x_axis_alpha =
        line_alpha(abs(in.world_pos.z), 0.01);

    let z_axis_alpha =
        line_alpha(abs(in.world_pos.x), 0.01);

    // X axis (red)
    colour = mix(
        colour,
        vec3<f32>(1.0, 0.0, 0.0),
        x_axis_alpha
    );

    // Z axis (blue)
    colour = mix(
        colour,
        vec3<f32>(0.0, 0.0, 1.0),
        z_axis_alpha
    );

    let alpha =
        max(
            max(minor_alpha, major_alpha),
            max(x_axis_alpha, z_axis_alpha)
        ) * fade;

    if (alpha <= 0.0) {
        discard;
    }

    return vec4<f32>(colour, alpha);
}
