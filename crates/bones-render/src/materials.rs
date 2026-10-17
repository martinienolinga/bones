//! Matériaux du maillage sur le GPU : un groupe de liaison par matériau
//! (couleur de base, texture, échantillonneur) et les plages d'indices à
//! dessiner avec chacun.

use std::ops::Range;

use bones_core::Mesh;
use bytemuck::{Pod, Zeroable};
use rayon::prelude::*;

use crate::textures::{self, Rgba};

/// Couleur « argile » des parties sans matériau (linéaire).
const CLAY: [f32; 4] = [0.62, 0.6, 0.58, 1.0];

/// Uniformes d'un matériau (voir `Material` en WGSL).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    base_color: [f32; 4],
    /// x : 1 si une texture est appliquée.
    flags: [f32; 4],
}

/// Disposition du groupe 1 : uniformes, texture, échantillonneur.
pub(crate) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty,
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("matériau"),
        entries: &[
            entry(
                0,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            entry(
                1,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            ),
            entry(2, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
        ],
    })
}

/// Ressources partagées par tous les matériaux.
pub(crate) struct MaterialContext {
    layout: wgpu::BindGroupLayout,
    white: wgpu::TextureView,
    sampler: wgpu::Sampler,
    /// Groupe des parties sans matériau.
    pub default: wgpu::BindGroup,
}

impl MaterialContext {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layout: wgpu::BindGroupLayout) -> Self {
        let white = textures::white(device, queue);
        let sampler = textures::sampler(device);
        let uniform = MaterialUniform {
            base_color: CLAY,
            flags: [0.0; 4],
        };
        let default = bind_group(device, &layout, uniform, &white, &sampler);
        Self {
            layout,
            white,
            sampler,
            default,
        }
    }
}

fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: MaterialUniform,
    texture: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    use wgpu::util::DeviceExt;
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("matériau"),
        contents: bytemuck::bytes_of(&uniform),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("matériau"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(texture),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

/// Matériaux d'un maillage et plages d'indices à dessiner.
pub(crate) struct Materials {
    groups: Vec<wgpu::BindGroup>,
    pub parts: Vec<(Range<u32>, Option<usize>)>,
}

impl Materials {
    /// Décode et envoie les textures en parallèle (un matériau par tâche) :
    /// chaque image décodée est libérée dès son envoi, ce qui borne la
    /// mémoire au nombre de fils de calcul.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, ctx: &MaterialContext, mesh: &Mesh) -> Self {
        let has_uvs = mesh.has_uvs();
        let views: Vec<Option<wgpu::TextureView>> = mesh
            .materials
            .par_iter()
            .map(|m| {
                let texture = m.base_color_texture.as_ref().filter(|_| has_uvs)?;
                match texture.decode_rgba8() {
                    Ok((width, height, pixels)) => {
                        let image = Rgba {
                            width,
                            height,
                            pixels,
                        };
                        Some(textures::upload(device, queue, &texture.name, image))
                    }
                    Err(e) => {
                        log::warn!("{} : {e}", texture.name);
                        None
                    }
                }
            })
            .collect();
        let groups = mesh
            .materials
            .iter()
            .zip(&views)
            .map(|(m, view)| {
                let uniform = MaterialUniform {
                    base_color: m.base_color,
                    flags: [f32::from(u8::from(view.is_some())), 0.0, 0.0, 0.0],
                };
                let texture = view.as_ref().unwrap_or(&ctx.white);
                bind_group(device, &ctx.layout, uniform, texture, &ctx.sampler)
            })
            .collect();
        let parts = mesh
            .parts()
            .into_iter()
            .map(|p| (p.indices, p.material.filter(|i| *i < mesh.materials.len())))
            .collect();
        Self { groups, parts }
    }

    /// Groupe de liaison d'une partie (matériau par défaut si aucun).
    pub fn group<'a>(&'a self, ctx: &'a MaterialContext, material: Option<usize>) -> &'a wgpu::BindGroup {
        material.and_then(|i| self.groups.get(i)).unwrap_or(&ctx.default)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn uniform_layout_matches_shader() {
        assert_eq!(std::mem::size_of::<super::MaterialUniform>(), 32);
    }
}
