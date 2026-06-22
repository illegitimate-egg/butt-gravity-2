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

var<workgroup> shared_positions: array<vec3<f32>, 64>;

fn calculate_force(position: vec3<f32>) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    for (var i = 0u; i < params.body_count; i++) {
        let secondary_position = shared_positions[i];
        
        if all(secondary_position.xyz == position.xyz) {
            continue;
        }
        
        let direction = secondary_position - position;
        let dist_sq = dot(direction, direction) + params.softening_sq;
        
        acc += direction * (1.0 / (dist_sq * sqrt(dist_sq)));
    }

    return acc;
}

@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) local_id: u32) {
    // We get blocked at the workgroupBarrier until the end of time, or until the GPU
    // catches onto our antics and the program terminates if we return early.
    // if (local_id >= params.body_count) { return; }
    let should_run = local_id < params.body_count;

    var p = input_buffer[local_id];
    let total_steps = c.step_target;

    let dt = params.dt;
    let y_c = 1.3512071917950166;
    let w_1 = y_c * dt;
    let w_0 = (1.0 - 2.0 * y_c) * dt;

    var dv = vec3<f32>(0.0);

    // for (var step = 0u; step < total_steps; step++) {
        // Triplet 1 using W1
        if should_run {
            shared_positions[local_id] = p.position_radius.xyz;
        } 
        workgroupBarrier();
        if should_run {
            dv = calculate_force(p.position_radius.xyz) * w_1;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
            p.position_radius.x += p.velocity_mass.x * w_1;
            p.position_radius.y += p.velocity_mass.y * w_1;
            p.position_radius.z += p.velocity_mass.z * w_1;
        }
        workgroupBarrier();
        if should_run {
            dv = calculate_force(p.position_radius.xyz) * w_1;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
        }
        
        // Triplet 2 using W0
        if should_run {
            shared_positions[local_id] = p.position_radius.xyz;
        } 
        workgroupBarrier();
        if should_run {
            dv = calculate_force(p.position_radius.xyz) * w_0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
            p.position_radius.x += p.velocity_mass.x * w_0;
            p.position_radius.y += p.velocity_mass.y * w_0;
            p.position_radius.z += p.velocity_mass.z * w_0;
        }
        workgroupBarrier();
        if should_run {
            dv = calculate_force(p.position_radius.xyz) * w_0;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
        }

        // Triplet 3 using W1
        if should_run {
            shared_positions[local_id] = p.position_radius.xyz;
        } 
        workgroupBarrier();
        if should_run {
            dv = calculate_force(p.position_radius.xyz) * w_1;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
            p.position_radius.x += p.velocity_mass.x * w_1;
            p.position_radius.y += p.velocity_mass.y * w_1;
            p.position_radius.z += p.velocity_mass.z * w_1;
        }
        workgroupBarrier();
        if should_run {
            dv = calculate_force(p.position_radius.xyz) * w_1;
            p.velocity_mass.x += dv.x;
            p.velocity_mass.y += dv.y;
            p.velocity_mass.z += dv.z;
        }
    // }

    if !should_run {
        return;
    }

    output_buffer[local_id] = p;
}
