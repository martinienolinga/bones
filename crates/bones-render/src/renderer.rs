//! Moteur de rendu : état GPU persistant et préparation d'une image. Le
//! dessin lui-même est dans `pass`.

use bones_core::{Mesh, Skin};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::bindings;
use crate::buffers::{GrowBuffer, MeshBuffers};
use crate::geometry::octahedron;
use crate::materials::{MaterialContext, Materials};
use crate::pipelines::Pipelines;
use crate::scene::{Globals, Scene};
use crate::targets::{RenderTargets, best_sample_count};

/// Ce que `prepare` a envoyé et que `render` doit dessiner.
#[derive(Default)]
pub(crate) struct Frame {
    pub bones: u32,
    pub lines: u32,
    pub overlay_lines: u32,
    pub show_mesh: bool,
    pub viewport: [f32; 4],
    pub background: [f64; 3],
}

pub struct Renderer {
    pub(crate) pipelines: Pipelines,
    globals: wgpu::Buffer,
    pub(crate) bind_group: wgpu::BindGroup,
    joints: GrowBuffer,
    pub(crate) octahedron: wgpu::Buffer,
    pub(crate) octahedron_vertices: u32,
    pub(crate) instances: GrowBuffer,
    pub(crate) lines: GrowBuffer,
    pub(crate) overlay_lines: GrowBuffer,
    pub(crate) mesh: Option<MeshBuffers>,
    /// Créé au premier maillage (il faut la file pour la texture blanche).
    pub(crate) material_context: Option<MaterialContext>,
    pub(crate) materials: Option<Materials>,
    pub(crate) targets: RenderTargets,
    pub(crate) frame: Frame,
    mesh_revision: u64,
    skin_revision: u64,
}

impl Renderer {
    /// Moteur de rendu avec MSAA 4× si le format le permet.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self::with_samples(device, format, best_sample_count(device, format))
    }

    /// Moteur de rendu avec un nombre d'échantillons imposé (1 : sans MSAA).
    pub fn with_samples(device: &wgpu::Device, format: wgpu::TextureFormat, samples: u32) -> Self {
        let pipelines = Pipelines::new(device, format, samples);
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let joints = GrowBuffer::new(device, "joints", wgpu::BufferUsages::STORAGE, 64 * 64);
        let bind_group = bindings::globals_group(device, &pipelines.bind_layout, &globals, &joints.buffer);
        let octa = octahedron();
        let octahedron = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("octahedron"),
            contents: bytemuck::cast_slice(&octa),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let vertex = wgpu::BufferUsages::VERTEX;
        Self {
            pipelines,
            globals,
            bind_group,
            joints,
            octahedron,
            octahedron_vertices: octa.len() as u32,
            instances: GrowBuffer::new(device, "bone-instances", vertex, 80 * 64),
            lines: GrowBuffer::new(device, "lines", vertex, 28 * 1024),
            overlay_lines: GrowBuffer::new(device, "overlay-lines", vertex, 28 * 1024),
            mesh: None,
            material_context: None,
            materials: None,
            targets: RenderTargets::new(format, samples),
            frame: Frame::default(),
            mesh_revision: 0,
            skin_revision: 0,
        }
    }

    /// Envoie le maillage, ses matériaux et ses textures au GPU si sa
    /// révision a changé.
    pub fn set_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mesh: &Mesh, revision: u64) {
        if revision == self.mesh_revision && self.mesh.is_some() {
            return;
        }
        self.mesh_revision = revision;
        self.skin_revision = 0;
        // Les anciennes ressources sont libérées avant d'envoyer les nouvelles.
        self.mesh = None;
        self.materials = None;
        self.mesh = MeshBuffers::upload(device, queue, mesh);
        if self.mesh.is_some() {
            let layout = &self.pipelines.material_layout;
            let ctx = self
                .material_context
                .get_or_insert_with(|| MaterialContext::new(device, queue, layout.clone()));
            self.materials = Some(Materials::new(device, queue, ctx, mesh));
        }
    }

    /// Envoie les poids au GPU si leur révision a changé.
    pub fn set_skin(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, skin: &Skin, revision: u64) {
        let Some(mesh) = &mut self.mesh else { return };
        if revision == self.skin_revision && mesh.weights.is_some() {
            return;
        }
        self.skin_revision = revision;
        mesh.upload_weights(device, queue, skin);
    }

    /// Prépare les données de l'image (uniformes, matrices, instances, lignes).
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) {
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&Globals::from_scene(scene)));
        let joints: &[u8] = if scene.joints.is_empty() {
            bytemuck::bytes_of(&Mat4::IDENTITY)
        } else {
            bytemuck::cast_slice(scene.joints)
        };
        if self.joints.write(device, queue, joints) {
            self.bind_group = bindings::globals_group(
                device,
                &self.pipelines.bind_layout,
                &self.globals,
                &self.joints.buffer,
            );
        }
        self.instances
            .write(device, queue, bytemuck::cast_slice(scene.bones));
        self.lines.write(device, queue, bytemuck::cast_slice(scene.lines));
        self.overlay_lines
            .write(device, queue, bytemuck::cast_slice(scene.overlay_lines));
        self.frame = Frame {
            bones: scene.bones.len() as u32,
            lines: scene.lines.len() as u32,
            overlay_lines: scene.overlay_lines.len() as u32,
            show_mesh: scene.show_mesh,
            viewport: scene.viewport,
            background: scene.background,
        };
    }
}
