//! Moteur de rendu : état GPU persistant et préparation d'une image. Le
//! dessin lui-même est dans `pass`.

use bones_core::{Mesh, Skin};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::bindings;
use crate::buffers::{GrowBuffer, MeshBuffers};
use crate::geometry::octahedron;
use crate::materials::{MaterialContext, Materials};
use crate::objects::ObjectCache;
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
    /// Arêtes du maillage par-dessus la carte de poids.
    pub wire: bool,
    pub viewport: [f32; 4],
    pub background: [f64; 3],
    /// Opacité du fond (0 : vignette sur fond transparent).
    pub clear_alpha: f64,
}

pub struct Renderer {
    pub(crate) pipelines: Pipelines,
    /// Un bloc `Globals` par élément (corps au décalage 0, puis un par objet),
    /// adressés par décalage dynamique.
    globals: GrowBuffer,
    /// Pas d'un bloc `Globals` (aligné sur le périphérique).
    globals_stride: u32,
    /// Blocs `Globals` assemblés sur le CPU, réutilisés d'une image à l'autre.
    globals_blob: Vec<u8>,
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
    pub(crate) objects: ObjectCache,
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
        let align = device.limits().min_uniform_buffer_offset_alignment.max(1);
        let globals_stride = (std::mem::size_of::<Globals>() as u32).next_multiple_of(align);
        let globals = GrowBuffer::new(
            device,
            "globals",
            wgpu::BufferUsages::UNIFORM,
            u64::from(globals_stride) * 8,
        );
        let joints = GrowBuffer::new(device, "joints", wgpu::BufferUsages::STORAGE, 64 * 64);
        let bind_group =
            bindings::globals_group(device, &pipelines.bind_layout, &globals.buffer, &joints.buffer);
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
            globals_stride,
            globals_blob: Vec::new(),
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
            objects: ObjectCache::default(),
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
        let joints: &[u8] = if scene.joints.is_empty() {
            bytemuck::bytes_of(&Mat4::IDENTITY)
        } else {
            bytemuck::cast_slice(scene.joints)
        };
        let joints_grew = self.joints.write(device, queue, joints);
        let globals_grew = self.prepare_globals(device, queue, scene);
        if joints_grew || globals_grew {
            self.bind_group = bindings::globals_group(
                device,
                &self.pipelines.bind_layout,
                &self.globals.buffer,
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
            wire: scene.weights.is_some_and(|w| w.wire),
            viewport: scene.viewport,
            background: scene.background,
            clear_alpha: 1.0,
        };
    }

    /// Met à jour les tampons des objets et assemble un bloc `Globals` par
    /// élément (corps puis objets). Retourne `true` si le tampon a grandi.
    fn prepare_globals(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) -> bool {
        // Contexte des matériaux créé au premier objet (il faut la file).
        if !scene.objects.is_empty() && self.material_context.is_none() {
            let layout = self.pipelines.material_layout.clone();
            self.material_context = Some(MaterialContext::new(device, queue, layout));
        }
        if let Some(ctx) = &self.material_context {
            self.objects.sync(
                device,
                queue,
                ctx,
                &self.pipelines.material_layout,
                scene.objects,
                self.globals_stride,
            );
        } else {
            self.objects.clear();
        }

        let size = std::mem::size_of::<Globals>();
        let stride = self.globals_stride as usize;
        self.globals_blob.clear();
        self.globals_blob.resize((1 + scene.objects.len()) * stride, 0);
        let base = Globals::from_scene(scene);
        self.globals_blob[..size].copy_from_slice(bytemuck::bytes_of(&base));
        for (i, draw) in scene.objects.iter().enumerate() {
            let skinned = self.objects.is_skinned(draw.id);
            let g = Globals::for_object(scene, draw, skinned);
            let off = (i + 1) * stride;
            self.globals_blob[off..off + size].copy_from_slice(bytemuck::bytes_of(&g));
        }
        self.globals.write(device, queue, &self.globals_blob)
    }
}
