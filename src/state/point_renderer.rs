use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, ComputePass, ComputePipeline, Device,
    RenderPass, RenderPipeline, TextureFormat,
};

use crate::{import::PlyData, points::Vertex};

pub struct PointRenderer {
    expander_bind_group_layout: BindGroupLayout,
    expander_bind_group: BindGroup,
    expander_pipeline: ComputePipeline,
    render_pipeline: RenderPipeline,
    point_vertices: Buffer,
    triangle_vertices: Buffer,
    num_points: u32,
}

impl PointRenderer {
    pub fn new(
        device: &Device,
        uniform_bind_group_layout: &BindGroupLayout,
        target_texture_format: TextureFormat,
    ) -> Self {
        // Create an initial dummy buffer for the point verticies.
        let point_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Vertex {
                position: [0.0, 0.0, 0.0],
                _padding0: [0; 4],
                color: [0.0, 0.0, 0.0],
                _padding1: [0; 4],
            }]),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create an initial dummy buffer for the triangle verticies.
        let triangle_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Triangle Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Vertex {
                position: [0.0, 0.0, 0.0],
                _padding0: [0; 4],
                color: [0.0, 0.0, 0.0],
                _padding1: [0; 4],
            }]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
        });

        // Create the bind group layout and the pipeline for the point to triangle expansion compute shader.
        let (expander_bind_group_layout, expander_pipeline) =
            point_expander_pipeline(device, uniform_bind_group_layout);

        // Create the inital bind group (this needs to be updated whenever the buffers are updated)
        // that binds the point and triangle verticies to be used in the expander pipeline.
        let expander_bind_group = expander_bind_group(
            device,
            &expander_bind_group_layout,
            &point_vertices,
            &triangle_vertices,
        );

        // Create the render pipeline which takes the triangles and renders them to draw the points (tiny triangles).
        let render_pipeline = render_pipeline(device, target_texture_format);

        Self {
            expander_bind_group_layout,
            expander_bind_group,
            expander_pipeline,
            render_pipeline,
            point_vertices,
            triangle_vertices,
            num_points: 0,
        }
    }

    pub fn compute<'a>(
        &'a self,
        compute_pass: &mut ComputePass<'a>,
        uniform_bind_group: &'a BindGroup,
    ) {
        if self.num_points != 0 {
            // Add the compute pipeline.
            compute_pass.set_pipeline(&self.expander_pipeline);
            compute_pass.set_bind_group(0, uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.expander_bind_group, &[]);
            compute_pass.dispatch((self.num_points as f64 / 64.0).ceil() as u32, 1, 1);
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.num_points != 0 {
            // Add the render pipeline.
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.triangle_vertices.slice(..));
            render_pass.draw(0..self.num_points * 3, 0..1);
        }
    }

    pub fn import_ply(&mut self, device: &Device, ply: &PlyData) {
        // Update the number of points and exit if there are none.
        self.num_points = ply.point_vertices.len() as u32;
        if self.num_points == 0 {
            return;
        }

        // Update the point verticies from the PLY data.
        let contents = bytemuck::cast_slice(&ply.point_vertices);
        self.point_vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Vertex Buffer"),
            contents,
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create a sufficiently large buffer for the triangle verticies.
        self.triangle_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Triangle Vertex Buffer"),
            size: (contents.len() * 3) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Since we just created brand new buffers, we need to recreate the bind group.
        self.expander_bind_group = expander_bind_group(
            device,
            &self.expander_bind_group_layout,
            &self.point_vertices,
            &self.triangle_vertices,
        );
    }
}

fn expander_bind_group(
    device: &Device,
    expander_bind_group_layout: &BindGroupLayout,
    point_vertices: &Buffer,
    triangle_vertices: &Buffer,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: expander_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: point_vertices.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: triangle_vertices.as_entire_binding(),
            },
        ],
        label: Some("Point Expander Bind Group"),
    })
}

fn point_expander_pipeline(
    device: &Device,
    uniform_bind_group_layout: &BindGroupLayout,
) -> (BindGroupLayout, ComputePipeline) {
    // Create the shader module.
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("Point Expander Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/point_expander.wgsl").into()),
    });

    // Create the layout of the expander bind group.
    let expander_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
            ],
            label: Some("Point Expander Bind Group Layout"),
        });

    // Create the pipeline layout.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Point Expander Pipeline Layout"),
        bind_group_layouts: &[uniform_bind_group_layout, &expander_bind_group_layout],
        push_constant_ranges: &[],
    });

    (
        expander_bind_group_layout,
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Point Expander Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        }),
    )
}

fn render_pipeline(device: &Device, target_texture_format: TextureFormat) -> RenderPipeline {
    // Create the shader module.
    let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("Point Rasterizer Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/point_rasterizer.wgsl").into()),
    });

    // Create the pipeline layout.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Point Rasterizer Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Point Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[target_texture_format.into()],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}
