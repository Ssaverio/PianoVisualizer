// note.rs
use bytemuck::{Pod, Zeroable};

// Rappresenta una singola nota
pub struct Note {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: [f32; 3],
}

// Vertice usato dalla GPU
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

impl Vertex {
    // Spiega alla GPU come leggere i dati
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
