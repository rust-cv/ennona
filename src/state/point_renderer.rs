use wgpu::{util::DeviceExt, BindGroup, Buffer, Device, RenderPass, RenderPipeline, TextureFormat};

use crate::{import::PlyData, points::Vertex};

pub struct PointRenderer {
    bind_group: BindGroup,
    render_pipeline: RenderPipeline,
    vertices: Buffer,
    num_points: u32,
}

impl PointRenderer {
    pub fn new(
        device: &Device,
        uniform_buffer: &Buffer,
        target_texture_format: TextureFormat,
    ) -> Self {
        // Create the shader module.
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Point Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/points.wgsl").into()),
        });

        // Create the layout of the bind group.
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Point Bind Group Layout"),
        });

        // Create the bind group itself, using the relevant buffers.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Point Bind Group"),
        });

        // Create the pipeline layout.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Point Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                topology: wgpu::PrimitiveTopology::PointList,
                polygon_mode: wgpu::PolygonMode::Point,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Vertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Store everything in the renderer.
        Self {
            bind_group,
            render_pipeline,
            vertices,
            num_points: 0,
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.num_points != 0 {
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertices.slice(..));
            render_pass.draw(0..self.num_points, 0..1);
        }
    }

    pub fn import_ply(&mut self, device: &Device, ply: &PlyData) {
        let contents = bytemuck::cast_slice(&ply.point_vertices);
        self.vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point Vertex Buffer"),
            contents,
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.num_points = ply.point_vertices.len() as u32;
    }
}
