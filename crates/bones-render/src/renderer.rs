//! Moteur de rendu : état GPU persistant et dessin d'une image.

use bones_core::{Mesh, Skin};
use glam::Mat4;
use wgpu::util::DeviceExt;

use crate::buffers::{GrowBuffer, MeshBuffers};
use crate::geometry::octahedron;
use crate::pipelines::Pipelines;
use crate::scene::{Globals, Scene};
use crate::targets::{RenderTargets, best_sample_count};

/// Partage de la plage de profondeur : la scène (grille, maillage) occupe
/// [SPLIT, 1] et la surcouche « rayons X » (os, poignées) [0, SPLIT]. La
/// surcouche passe ainsi toujours devant le maillage tout en gardant un test
/// de profondeur entre os, le tout dans une seule passe — ce qui permet des
/// attachements sans mémoire sur iPad.
const OVERLAY_DEPTH_SPLIT: f32 = 0.1;

/// Ce que `prepare` a envoyé et que `render` doit dessiner.
#[derive(Default)]
struct Frame {
    bones: u32,
    lines: u32,
    overlay_lines: u32,
    show_mesh: bool,
    viewport: [f32; 4],
    background: [f64; 3],
}

pub struct Renderer {
    pipelines: Pipelines,
    globals: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    joints: GrowBuffer,
    octahedron: wgpu::Buffer,
    octahedron_vertices: u32,
    instances: GrowBuffer,
    lines: GrowBuffer,
    overlay_lines: GrowBuffer,
    mesh: Option<MeshBuffers>,
    targets: RenderTargets,
    frame: Frame,
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
        let bind_group = pipelines.bind_group(device, &globals, &joints.buffer);
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
            targets: RenderTargets::new(format, samples),
            frame: Frame::default(),
            mesh_revision: 0,
            skin_revision: 0,
        }
    }

    /// Envoie le maillage au GPU si sa révision a changé.
    pub fn set_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mesh: &Mesh, revision: u64) {
        if revision == self.mesh_revision && self.mesh.is_some() {
            return;
        }
        self.mesh_revision = revision;
        self.skin_revision = 0;
        self.mesh = MeshBuffers::upload(device, queue, mesh);
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
            self.bind_group = self
                .pipelines
                .bind_group(device, &self.globals, &self.joints.buffer);
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

    /// Dessine la scène préparée dans `target` (taille `size` en pixels), en
    /// une seule passe.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
    ) {
        let views = self.targets.views(device, size);
        let f = &self.frame;
        let [x, y, w, h] = clamp_viewport(f.viewport, size);
        let [r, g, b] = f.background;
        let (view, resolve_target, store) = match &views.msaa {
            Some(msaa) => (msaa, Some(target), wgpu::StoreOp::Discard),
            None => (target, None, wgpu::StoreOp::Store),
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                    store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &views.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.bind_group, &[]);

        // Scène : maillage puis grille.
        pass.set_viewport(x, y, w, h, OVERLAY_DEPTH_SPLIT, 1.0);
        if f.show_mesh
            && let Some(mesh) = &self.mesh
            && let Some(weights) = &mesh.weights
        {
            pass.set_pipeline(&self.pipelines.mesh);
            pass.set_vertex_buffer(0, mesh.positions.slice(..));
            pass.set_vertex_buffer(1, mesh.normals.slice(..));
            pass.set_vertex_buffer(2, weights.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
        if f.lines > 0 {
            pass.set_pipeline(&self.pipelines.line);
            pass.set_vertex_buffer(0, self.lines.buffer.slice(..));
            pass.draw(0..f.lines, 0..1);
        }

        // Surcouche « rayons X » : os, poignées, trajectoires.
        pass.set_viewport(x, y, w, h, 0.0, OVERLAY_DEPTH_SPLIT);
        if f.bones > 0 {
            pass.set_pipeline(&self.pipelines.bone);
            pass.set_vertex_buffer(0, self.octahedron.slice(..));
            pass.set_vertex_buffer(1, self.instances.buffer.slice(..));
            pass.draw(0..self.octahedron_vertices, 0..f.bones);
        }
        if f.overlay_lines > 0 {
            pass.set_pipeline(&self.pipelines.overlay_line);
            pass.set_vertex_buffer(0, self.overlay_lines.buffer.slice(..));
            pass.draw(0..f.overlay_lines, 0..1);
        }
    }
}

/// Restreint la zone de rendu à la cible (wgpu refuse un viewport qui déborde).
fn clamp_viewport(viewport: [f32; 4], size: [u32; 2]) -> [f32; 4] {
    let [x, y, w, h] = viewport;
    let (sw, sh) = (size[0] as f32, size[1] as f32);
    let (x, y) = (x.clamp(0.0, sw - 1.0), y.clamp(0.0, sh - 1.0));
    [x, y, w.min(sw - x).max(1.0), h.min(sh - y).max(1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_is_clamped_to_target() {
        assert_eq!(
            clamp_viewport([0.0, 0.0, 800.0, 600.0], [800, 600]),
            [0.0, 0.0, 800.0, 600.0]
        );
        assert_eq!(
            clamp_viewport([-10.0, 50.0, 900.0, 600.0], [800, 600]),
            [0.0, 50.0, 800.0, 550.0]
        );
        assert_eq!(
            clamp_viewport([0.0, 0.0, 0.0, 0.0], [800, 600]),
            [0.0, 0.0, 1.0, 1.0]
        );
    }
}
