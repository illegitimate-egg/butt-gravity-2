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
    if (body_id > parameters.body_count) {
        return;
    }

    var acceleration = vec3<f32>(0.0);
    for (var j: u32 = 0u; j < parameters.body_count; j++) {
        if (body_id == j) {
            continue;
        }

        let secondary = input_buffer[j];

        let r = secondary.position_radius.xyz - input_buffer[body_id].position_radius.xyz;
        let dist_sq = dot(r, r) + parameters.softening;

        let inv_dist = inverseSqrt(dist_sq);
        let inv_dist_3 = inv_dist * inv_dist * inv_dist;

        acceleration += G * secondary.velocity_mass.w * r * inv_dist_3;
    }

    output_buffer[body_id] = input_buffer[body_id];
    output_buffer[body_id].velocity_mass.x += acceleration.x * parameters.dt;
    output_buffer[body_id].velocity_mass.y += acceleration.y * parameters.dt;
    output_buffer[body_id].velocity_mass.z += acceleration.z * parameters.dt;
}
