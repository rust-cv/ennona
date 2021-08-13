use wgpu::{
    util::DeviceExt, Buffer, Device, PipelineLayout, RenderPipeline, ShaderModule,
    SwapChainDescriptor,
};

use crate::{import::PlyData, state::Vertex};

#[derive(Debug)]
pub struct Face {
    pub vertex_index: Vec<i32>,
}

pub struct FaceState {
    pub pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub num_indices: u32,
}

impl FaceState {
    pub fn new(
        sc_desc: &SwapChainDescriptor,
        device: &Device,
        render_pipeline_layout: &PipelineLayout,
        point_shader: &ShaderModule,
    ) -> Self {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Face Render Pipeline"),
            layout: Some(render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: point_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: point_shader,
                entry_point: "fs_main",
                targets: &[sc_desc.format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Vertex Buffer"),
            contents: bytemuck::cast_slice(&[Vertex {
                position: [0.0, 0.0, 0.0],
                color: [0.0, 0.0, 0.0],
            }]),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Index Buffer"),
            contents: &[],
            usage: wgpu::BufferUsage::INDEX,
        });
        let num_indices = 0;
        Self {
            vertex_buffer,
            index_buffer,
            num_indices,
            pipeline,
        }
    }

    pub fn import_faces(&mut self, device: &Device, ply: &PlyData) {
        self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Vertex Buffer"),
            contents: bytemuck::cast_slice(&ply.face_vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });
        self.num_indices = ply.face_indices.len() as u32;
        self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Face Index Buffer"),
            contents: bytemuck::cast_slice(&ply.face_indices),
            usage: wgpu::BufferUsage::INDEX,
        });
    }
}
