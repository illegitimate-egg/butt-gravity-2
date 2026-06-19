struct CameraUniform {
    v: mat4x4<f32>,
    vp: mat4x4<f32>,
}

struct BodyState {
    position_radius: vec4<f32>,
    velocity_mass: vec4<f32>,
    color: vec4<f32>,
}

struct SimParams {
    dt: f32,
    body_count: u32,
    _pad: vec2<u32>
}

@group(0) @binding(0) var<storage, read> bodies: array<BodyState>;
// DO NOT USE, This is the write binding for the compute shader
// @group(0) @binding(1) var<storage, read_write> _out: array<BodyState>;
@group(0) @binding(2) var<uniform> parameters: SimParams;

@group(1) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

const vertex_positions = array<f32, 12>(-1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0);

@vertex
fn vs_main(@builtin(vertex_index) vtx: u32,
           @builtin(instance_index) inst: u32) -> VertexOutput {
    let body = bodies[inst];

    let pos = body.position_radius.xyz;
    let radius = body.position_radius.w;

    let corner = vec2<f32>(vertex_positions[vtx * 2], vertex_positions[vtx * 2 + 1]);

    let right = vec3<f32>(camera.v[0][0], camera.v[1][0], camera.v[2][0]);
    let up    = vec3<f32>(camera.v[0][1], camera.v[1][1], camera.v[2][1]);

    let world = pos + (corner.x * right + corner.y * up) * radius;

    var out: VertexOutput;
    out.position = camera.vp * vec4<f32>(world, 1.0);
    out.uv = corner * 0.5 + vec2<f32>(0.5);
    out.color = body.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let centered = in.uv * 2.0 - vec2<f32>(1.0);

    let dist = length(centered);

    // soft edge (anti-alias)
    let alpha = 1.0 - smoothstep(0.95, 1.0, dist);

    if (alpha <= 0.0) {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
