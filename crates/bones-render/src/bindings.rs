//! Groupe de liaison 0 : uniformes globaux et matrices des os, partagés par
//! tous les pipelines.
//!
//! Les uniformes utilisent un **décalage dynamique** : un bloc `Globals` par
//! élément (corps et surcouche au décalage 0, puis un par objet) dans un même
//! tampon ; `set_bind_group` choisit le bloc. Les matrices des os (binding 1)
//! restent partagées par tous.

use std::num::NonZeroU64;

use crate::scene::Globals;

/// Disposition du groupe 0.
pub(crate) fn globals_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("globals"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<Globals>() as u64),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}

/// Groupe 0 : à recréer quand l'un des tampons grandit. Le binding 0 n'expose
/// qu'un bloc `Globals` à la fois (taille fixe, décalage dynamique).
pub(crate) fn globals_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    globals: &wgpu::Buffer,
    joints: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("globals"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: globals,
                    offset: 0,
                    size: NonZeroU64::new(std::mem::size_of::<Globals>() as u64),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: joints.as_entire_binding(),
            },
        ],
    })
}
