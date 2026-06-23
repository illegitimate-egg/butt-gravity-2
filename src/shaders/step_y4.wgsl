struct BodyState {
    position_radius: vec4<f32>,
    velocity_mass: vec4<f32>,
    color: vec4<f32>,
}

struct SimParams {
    dt: f32,
    body_count: u32,
    softening_sq: f32,
    _pad: u32,
}

struct Immediates {
    step_target: u32,
}

var<immediate> c: Immediates;

@group(0) @binding(0) var<storage, read> input_buffer: array<BodyState>;
@group(0) @binding(1) var<storage, read_write> output_buffer: array<BodyState>;
@group(0) @binding(2) var<uniform> params: SimParams;

/// XYZ: Position, W: Mass
var<workgroup> shared_positions: array<vec4<f32>, 64>;

const G: f32 = 6.67430e-11;

fn calculate_acceleration(position: vec3<f32>, p_index: u32) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    for (var i = 0u; i < params.body_count; i++) {
        if (p_index == i) {
            continue;
        }

        let secondary = shared_positions[i];

        // softening_sq = eps2
        let r = secondary.xyz - position;
        let dist_sq = dot(r, r) + params.softening_sq;

        let inv_dist = inverseSqrt(dist_sq);
        let inv_dist_3 = inv_dist * inv_dist * inv_dist;
        
        acc += G * secondary.w * r * inv_dist_3;
    }

    return acc;
}

@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) local_id: u32) {
    // If we return early, we get blocked at the wokgroupBarrier until the collapse of society,
    // or until the GPU decides that it's had enough of our antics (blocking for
    // several seconds).
    // if (local_id >= params.body_count) { return; }
    let should_run = local_id < params.body_count;

    var p = input_buffer[local_id];
    let total_steps = c.step_target;

    let dt = params.dt;
    let y_c = 1.3512071917950166;
    let w_1 = y_c * dt;
    let w_0 = (1.0 - 2.0 * y_c) * dt;

    var dv = vec3<f32>(0.0);

    for (var step = 0u; step < total_steps; step++) {
        // Triplet 1 using W1
        if should_run {
            shared_positions[local_id].x = p.position_radius.x;
            shared_positions[local_id].y = p.position_radius.y;
            shared_positions[local_id].z = p.position_radius.z;
            shared_positions[local_id].w = p.velocity_mass.w;
        } 
        workgroupBarrier();
        if should_run {
            // K
            dv = calculate_acceleration(p.position_radius.xyz, local_id) * w_1 / 2.0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
            // D
            p.position_radius.x += p.velocity_mass.x * w_1;
            p.position_radius.y += p.velocity_mass.y * w_1;
            p.position_radius.z += p.velocity_mass.z * w_1;
        }
        if should_run {
            shared_positions[local_id].x = p.position_radius.x;
            shared_positions[local_id].y = p.position_radius.y;
            shared_positions[local_id].z = p.position_radius.z;
            shared_positions[local_id].w = p.velocity_mass.w;
        } 
        workgroupBarrier();
        if should_run {
            // K
            dv = calculate_acceleration(p.position_radius.xyz, local_id) * w_1 / 2.0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
        }
        
        // Triplet 2 using W0
        if should_run {
            shared_positions[local_id].x = p.position_radius.x;
            shared_positions[local_id].y = p.position_radius.y;
            shared_positions[local_id].z = p.position_radius.z;
            shared_positions[local_id].w = p.velocity_mass.w;
        } 
        workgroupBarrier();
        if should_run {
            dv = calculate_acceleration(p.position_radius.xyz, local_id) * w_0 / 2.0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
            p.position_radius.x += p.velocity_mass.x * w_0;
            p.position_radius.y += p.velocity_mass.y * w_0;
            p.position_radius.z += p.velocity_mass.z * w_0;
        }
        if should_run {
            shared_positions[local_id].x = p.position_radius.x;
            shared_positions[local_id].y = p.position_radius.y;
            shared_positions[local_id].z = p.position_radius.z;
            shared_positions[local_id].w = p.velocity_mass.w;
        } 
        workgroupBarrier();
        if should_run {
            dv = calculate_acceleration(p.position_radius.xyz, local_id) * w_0 / 2.0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
        }

        // Triplet 3 using W1
        if should_run {
            shared_positions[local_id].x = p.position_radius.x;
            shared_positions[local_id].y = p.position_radius.y;
            shared_positions[local_id].z = p.position_radius.z;
            shared_positions[local_id].w = p.velocity_mass.w;
        } 
        workgroupBarrier();
        if should_run {
            dv = calculate_acceleration(p.position_radius.xyz, local_id) * w_1 / 2.0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
            p.position_radius.x += p.velocity_mass.x * w_1;
            p.position_radius.y += p.velocity_mass.y * w_1;
            p.position_radius.z += p.velocity_mass.z * w_1;
        }
        if should_run {
            shared_positions[local_id].x = p.position_radius.x;
            shared_positions[local_id].y = p.position_radius.y;
            shared_positions[local_id].z = p.position_radius.z;
            shared_positions[local_id].w = p.velocity_mass.w;
        } 
        workgroupBarrier();
        if should_run {
            dv = calculate_acceleration(p.position_radius.xyz, local_id) * w_1 / 2.0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
        }
    }

    if !should_run {
        return;
    }

    output_buffer[local_id] = p;
}
