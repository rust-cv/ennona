use wgpu::{
    util::DeviceExt, BindGroup, BindGroupLayout, Buffer, Device, PipelineLayout, RenderPass,
    RenderPipeline, ShaderModule, TextureFormat,
};

use crate::{import::PlyData, points::Vertex};

pub struct FaceRenderer {
    shader: ShaderModule,
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline_layout: PipelineLayout,
    render_pipeline: RenderPipeline,
    vertices: Buffer,
    indices: Buffer,
    num_indices: u32,
}

impl FaceRenderer {
    pub fn new(
        device: &Device,
        uniform_buffer: &Buffer,
        target_texture_format: TextureFormat,
    ) -> Self {
        // Create the shader module.
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Face Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/faces.wgsl").into()),
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
            label: Some("Face Bind Group Layout"),
        });

        // Create the bind group itself, using the relevant buffers.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Face Bind Group"),
        });

        // Create the pipeline layout.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Face Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Face Render Pipeline"),
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
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        // Create the buffer for the face verticies.
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Vertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 0.0],
            }]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create the buffer for the face indices.
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Index Buffer"),
            contents: &[],
            usage: wgpu::BufferUsages::INDEX,
        });

        // Store everything in the renderer.
        Self {
            shader,
            bind_group_layout,
            bind_group,
            pipeline_layout,
            render_pipeline,
            vertices,
            indices,
            num_indices: 0,
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.num_indices != 0 {
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertices.slice(..));
            render_pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }
    }

    pub fn import_ply(&mut self, device: &Device, ply: &PlyData) {
        self.vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Vertex Buffer"),
            contents: bytemuck::cast_slice(&ply.face_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        self.num_indices = ply.face_indices.len() as u32;
        self.indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Index Buffer"),
            contents: bytemuck::cast_slice(&ply.face_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
    }
}
