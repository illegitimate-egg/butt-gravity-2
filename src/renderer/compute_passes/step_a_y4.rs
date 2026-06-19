use wgpu::util::DeviceExt;
use wgpu::BufferUsages;

// Correlates with number in shader
const BODIES_PER_GROUP: u32 = 64;

pub struct Y4AccelerationStep {
    sim_buffer_a: wgpu::Buffer,
    sim_buffer_b: wgpu::Buffer,

    pub sim_params: SimParams,
    sim_params_buffer: wgpu::Buffer,

    compute_pipeline: wgpu::ComputePipeline,
    pub compute_layout: wgpu::BindGroupLayout,

    /// False -> 0 -> AB
    /// True  -> 1 -> BA
    pub swap_buffers: bool,

    pub ab_bind_group: wgpu::BindGroup,
    pub ba_bind_group: wgpu::BindGroup,
}

impl Y4AccelerationStep {
    pub fn new(device: &wgpu::Device, initial_bodies: Vec<BodyState>) -> Self {
        let compute_shader =
            device.create_shader_module(wgpu::include_wgsl!("../../shaders/step_a_y4.wgsl"));

        let sim_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("y4 Acceleration Sim Data A"),
            contents: bytemuck::cast_slice(&initial_bodies),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        });
        let sim_buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("y4 Acceleration Sim Data B"),
            size: (initial_bodies.len() * std::mem::size_of::<BodyState>()) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let initial_sim_params = SimParams {
            // 240Hz
            dt: 1.0 / 240.0,
            body_count: initial_bodies.len() as u32,
            _pad: [0; 2],
        };
        let sim_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("y4 Acceleration Parameters"),
            contents: bytemuck::bytes_of(&initial_sim_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("y4 Acceleration Bind Group Layout"),
            entries: &[
                // input_bodies
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // output_bodies
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let ab_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("y4 Acceleration A->B Bind Group"),
            layout: &compute_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sim_params_buffer.as_entire_binding(),
                },
            ],
        });
        let ba_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("y4 Acceleration B->A Bind Group"),
            layout: &compute_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sim_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sim_params_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("y4 Acceleration Pipeline Layout"),
                bind_group_layouts: &[Some(&compute_layout)],
                immediate_size: 0,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("y4 Acceleration Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            sim_buffer_a,
            sim_buffer_b,
            sim_params: initial_sim_params,
            sim_params_buffer,
            swap_buffers: false,
            compute_pipeline,
            compute_layout,
            ab_bind_group,
            ba_bind_group,
        }
    }

    pub fn run(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("y4 Acceleration Compute Pass"),
            timestamp_writes: None,
        });

        if !self.swap_buffers {
            pass.set_bind_group(0, &self.ab_bind_group, &[]);
        } else {
            pass.set_bind_group(0, &self.ba_bind_group, &[]);
        }
        self.swap_buffers = !self.swap_buffers;

        pass.set_pipeline(&self.compute_pipeline);
        pass.dispatch_workgroups(
            ((self.sim_params.body_count as f32) / (BODIES_PER_GROUP as f32)).ceil() as u32,
            1,
            1,
        );
    }
}

// GPU Types
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BodyState {
    // X, Y, Z, Radius
    pub position_radius: [f32; 4],
    // dX/dt, dY/dt, dZ/dt, Mass
    pub velocity_mass: [f32; 4],

    // Colour
    // sRGB(A)
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimParams {
    pub dt: f32,
    pub body_count: u32,

    // Bring the usage up to a multiple of 16:
    // 4 + 4 + 4 * 2 = 16
    pub _pad: [u32; 2],
}
