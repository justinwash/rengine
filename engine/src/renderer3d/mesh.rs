use crate::assets::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshId(pub(crate) usize);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex3D {
    pub fn new(position: [f32; 3], normal: [f32; 3], color: Color) -> Self {
        Self {
            position,
            normal,
            color: color.to_array(),
        }
    }

    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub fn cube_mesh(sx: f32, sy: f32, sz: f32, color: Color) -> (Vec<Vertex3D>, Vec<u32>) {
    let hx = sx / 2.0;
    let hy = sy / 2.0;
    let hz = sz / 2.0;
    let c = color.to_array();

    let mut verts = Vec::with_capacity(24);
    let mut idxs = Vec::with_capacity(36);

    let mut add_face = |positions: [[f32; 3]; 4], normal: [f32; 3]| {
        let base = verts.len() as u32;
        for p in &positions {
            verts.push(Vertex3D {
                position: *p,
                normal,
                color: c,
            });
        }
        idxs.extend_from_slice(&[base + 2, base + 1, base, base, base + 3, base + 2]);
    };

    add_face(
        [[-hx, hy, -hz], [hx, hy, -hz], [hx, hy, hz], [-hx, hy, hz]],
        [0.0, 1.0, 0.0],
    );

    add_face(
        [
            [-hx, -hy, hz],
            [hx, -hy, hz],
            [hx, -hy, -hz],
            [-hx, -hy, -hz],
        ],
        [0.0, -1.0, 0.0],
    );

    add_face(
        [[-hx, -hy, hz], [-hx, hy, hz], [hx, hy, hz], [hx, -hy, hz]],
        [0.0, 0.0, 1.0],
    );

    add_face(
        [
            [hx, -hy, -hz],
            [hx, hy, -hz],
            [-hx, hy, -hz],
            [-hx, -hy, -hz],
        ],
        [0.0, 0.0, -1.0],
    );

    add_face(
        [[hx, -hy, hz], [hx, hy, hz], [hx, hy, -hz], [hx, -hy, -hz]],
        [1.0, 0.0, 0.0],
    );

    add_face(
        [
            [-hx, -hy, -hz],
            [-hx, hy, -hz],
            [-hx, hy, hz],
            [-hx, -hy, hz],
        ],
        [-1.0, 0.0, 0.0],
    );

    (verts, idxs)
}

pub fn floor_quad(width: f32, depth: f32, y: f32, color: Color) -> (Vec<Vertex3D>, Vec<u32>) {
    let hw = width / 2.0;
    let hd = depth / 2.0;
    let c = color.to_array();
    let n = [0.0, 1.0, 0.0];
    let verts = vec![
        Vertex3D {
            position: [-hw, y, -hd],
            normal: n,
            color: c,
        },
        Vertex3D {
            position: [hw, y, -hd],
            normal: n,
            color: c,
        },
        Vertex3D {
            position: [hw, y, hd],
            normal: n,
            color: c,
        },
        Vertex3D {
            position: [-hw, y, hd],
            normal: n,
            color: c,
        },
    ];
    let idxs = vec![2, 1, 0, 0, 3, 2];
    (verts, idxs)
}

pub fn wall_quad(
    p0: [f32; 3],
    p1: [f32; 3],
    height: f32,
    color: Color,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let c = color.to_array();

    let dx = p1[0] - p0[0];
    let dz = p1[2] - p0[2];
    let len = (dx * dx + dz * dz).sqrt();
    let n = [-dz / len, 0.0, dx / len];

    let verts = vec![
        Vertex3D {
            position: p0,
            normal: n,
            color: c,
        },
        Vertex3D {
            position: p1,
            normal: n,
            color: c,
        },
        Vertex3D {
            position: [p1[0], p1[1] + height, p1[2]],
            normal: n,
            color: c,
        },
        Vertex3D {
            position: [p0[0], p0[1] + height, p0[2]],
            normal: n,
            color: c,
        },
    ];
    let idxs = vec![0, 1, 2, 2, 3, 0];
    (verts, idxs)
}
