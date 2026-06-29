use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x3,
        2 => Float32x4,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct MeshBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl MeshBuffer {
    pub fn new(device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mesh Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        MeshBuffer { vertex_buffer, index_buffer, num_indices: indices.len() as u32 }
    }

    pub fn cube(device: &wgpu::Device) -> Self {
        let vertices = [
            Vertex {
                position: [-0.5, -0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
                color: [0.8, 0.2, 0.2, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
                color: [0.8, 0.2, 0.2, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
                color: [0.8, 0.2, 0.2, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                normal: [0.0, 0.0, 1.0],
                color: [0.8, 0.2, 0.2, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
                color: [0.2, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
                color: [0.2, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
                color: [0.2, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5],
                normal: [0.0, 0.0, -1.0],
                color: [0.2, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5],
                normal: [0.0, 1.0, 0.0],
                color: [0.2, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5],
                normal: [0.0, 1.0, 0.0],
                color: [0.2, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
                color: [0.2, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
                color: [0.2, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, -0.5],
                normal: [0.0, -1.0, 0.0],
                color: [0.8, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, -0.5],
                normal: [0.0, -1.0, 0.0],
                color: [0.8, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                normal: [0.0, -1.0, 0.0],
                color: [0.8, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5],
                normal: [0.0, -1.0, 0.0],
                color: [0.8, 0.8, 0.2, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, -0.5],
                normal: [1.0, 0.0, 0.0],
                color: [0.8, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, -0.5],
                normal: [1.0, 0.0, 0.0],
                color: [0.8, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [0.5, 0.5, 0.5],
                normal: [1.0, 0.0, 0.0],
                color: [0.8, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [0.5, -0.5, 0.5],
                normal: [1.0, 0.0, 0.0],
                color: [0.8, 0.2, 0.8, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, -0.5],
                normal: [-1.0, 0.0, 0.0],
                color: [0.2, 0.8, 0.8, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, -0.5],
                normal: [-1.0, 0.0, 0.0],
                color: [0.2, 0.8, 0.8, 1.0],
            },
            Vertex {
                position: [-0.5, 0.5, 0.5],
                normal: [-1.0, 0.0, 0.0],
                color: [0.2, 0.8, 0.8, 1.0],
            },
            Vertex {
                position: [-0.5, -0.5, 0.5],
                normal: [-1.0, 0.0, 0.0],
                color: [0.2, 0.8, 0.8, 1.0],
            },
        ];

        let indices: Vec<u32> = vec![
            0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16,
            17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23,
        ];

        Self::new(device, &vertices, &indices)
    }

    pub fn sphere(device: &wgpu::Device, segments: u32, rings: u32) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for ring in 0..=rings {
            let phi = std::f32::consts::PI * ring as f32 / rings as f32;
            for segment in 0..=segments {
                let theta = 2.0 * std::f32::consts::PI * segment as f32 / segments as f32;
                let x = phi.sin() * theta.cos();
                let y = phi.cos();
                let z = phi.sin() * theta.sin();
                vertices.push(Vertex {
                    position: [x, y, z],
                    normal: [x, y, z],
                    color: [0.6, 0.6, 0.8, 1.0],
                });
            }
        }

        for ring in 0..rings {
            for segment in 0..segments {
                let a = ring * (segments + 1) + segment;
                let b = a + segments + 1;
                indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
            }
        }

        Self::new(device, &vertices, &indices)
    }
}
