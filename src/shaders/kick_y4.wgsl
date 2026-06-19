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

struct Immediates {
    effective_dt: f32,
}
var<immediate> c: Immediates;

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

    let primary = input_buffer[body_id];

    var acceleration = vec3<f32>(0.0);
    for (var j: u32 = 0u; j < parameters.body_count; j++) {
        if (body_id == j) {
            continue;
        }

        let secondary = input_buffer[j];

        let eps2 = parameters.softening * parameters.softening;
        let r = secondary.position_radius.xyz - primary.position_radius.xyz;
        let dist_sq = dot(r, r) + eps2;

        let inv_dist = inverseSqrt(dist_sq);
        let inv_dist_3 = inv_dist * inv_dist * inv_dist;

        acceleration += G * secondary.velocity_mass.w * r * inv_dist_3;
    }

    output_buffer[body_id] = input_buffer[body_id];
    output_buffer[body_id].velocity_mass.x += acceleration.x * c.effective_dt;
    output_buffer[body_id].velocity_mass.y += acceleration.y * c.effective_dt;
    output_buffer[body_id].velocity_mass.z += acceleration.z * c.effective_dt;
}
