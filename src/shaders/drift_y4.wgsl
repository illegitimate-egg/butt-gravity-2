struct BodyState {
    position_radius: vec4<f32>,
    velocity_mass: vec4<f32>,
    color: vec4<f32>,
}

struct SimParams {
    dt: f32,
    body_count: u32,
    softening: f32,
    _pad: u32
}

@group(0) @binding(0) var<storage, read> input_buffer: array<BodyState>;
@group(0) @binding(1) var<storage, read_write> output_buffer: array<BodyState>;
@group(0) @binding(2) var<uniform> parameters: SimParams;

const G: f32 = 6.67430e-11;

@compute
// Should correlate with BODIES_PER_GROUP in render pass
@workgroup_size(64)
fn cs_main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>
) {
    let body_id = global_invocation_id.x;
    if (body_id >= parameters.body_count) {
        return;
    }

    output_buffer[body_id] = input_buffer[body_id];
    output_buffer[body_id].position_radius.x += input_buffer[body_id].velocity_mass.x * parameters.dt;
    output_buffer[body_id].position_radius.y += input_buffer[body_id].velocity_mass.y * parameters.dt;
    output_buffer[body_id].position_radius.z += input_buffer[body_id].velocity_mass.z * parameters.dt;
}

