//! Dispositions des tampons de sommets (doivent correspondre aux `@location` WGSL).

use crate::scene::BoneInstance;

const POSITION_ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
const NORMAL_ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![1 => Float32x3];
const WEIGHT_ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![2 => Uint16x4, 3 => Unorm16x4];
const UV_ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![4 => Float32x2];
const OCTAHEDRON_ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
const INSTANCE_ATTRS: [wgpu::VertexAttribute; 5] =
    wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4];
pub(crate) const LINE_ATTRS: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];

/// Positions, normales, poids et coordonnées de texture : quatre tampons
/// séparés.
pub(crate) fn mesh_buffers() -> [Option<wgpu::VertexBufferLayout<'static>>; 4] {
    [
        Some(wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &POSITION_ATTRS,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &NORMAL_ATTRS,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<bones_core::VertexWeights>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &WEIGHT_ATTRS,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: 8,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &UV_ATTRS,
        }),
    ]
}

/// Octaèdre (position + normale) et instances (matrice + couleur).
pub(crate) fn bone_buffers() -> [Option<wgpu::VertexBufferLayout<'static>>; 2] {
    [
        Some(wgpu::VertexBufferLayout {
            array_stride: 24,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &OCTAHEDRON_ATTRS,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BoneInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRS,
        }),
    ]
}
