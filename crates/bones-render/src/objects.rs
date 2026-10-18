//! Objets supplémentaires de la scène (accessoires, vêtements, pièces) sur le
//! GPU : un cache de tampons par objet, clavé sur les révisions du maillage et
//! des poids, et le dessin de ces objets dans la passe de scène.
//!
//! Comme le corps, un objet n'est renvoyé au GPU que lorsque sa révision
//! change ; les objets disparus voient leurs tampons libérés. Un objet rigide
//! est dessiné par le pipeline `object` (matrice du modèle seule), un objet
//! déformé par le pipeline `mesh` (placement puis palette d'os du corps).

use std::collections::HashMap;

use bones_core::{ObjectDraw, ObjectId};

use crate::buffers::MeshBuffers;
use crate::materials::{MaterialContext, Materials};
use crate::renderer::Renderer;

/// Tampons GPU d'un objet et leurs révisions.
struct ObjectGpu {
    mesh: MeshBuffers,
    materials: Materials,
    mesh_revision: u64,
    skin_revision: u64,
    /// Déformé par le squelette (poids présents) ou rigide.
    skinned: bool,
}

/// Cache des objets résidents sur le GPU, avec leur ordre de dessin.
#[derive(Default)]
pub(crate) struct ObjectCache {
    gpu: HashMap<ObjectId, ObjectGpu>,
    /// Objets à dessiner cette image : identifiant et décalage dynamique du
    /// bloc `Globals` correspondant.
    order: Vec<(ObjectId, u32)>,
}

impl ObjectCache {
    /// Met à jour les tampons des objets `draws` (envois seulement si une
    /// révision a changé), libère ceux qui ont disparu et note leur ordre de
    /// dessin. `stride` : pas d'un bloc `Globals`, `base` : décalage du
    /// premier objet (le corps occupe le bloc 0).
    pub fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ctx: &MaterialContext,
        material_layout: &wgpu::BindGroupLayout,
        draws: &[ObjectDraw],
        stride: u32,
    ) {
        self.order.clear();
        for (i, draw) in draws.iter().enumerate() {
            if !self.upload(device, queue, ctx, draw) {
                continue;
            }
            let _ = material_layout; // Groupe déjà construit via `ctx`.
            self.order.push((draw.id, (i as u32 + 1) * stride));
        }
        let alive: Vec<ObjectId> = draws.iter().map(|d| d.id).collect();
        self.gpu.retain(|id, _| alive.contains(id));
    }

    /// Oublie l'ordre de dessin (aucun objet cette image).
    pub fn clear(&mut self) {
        self.order.clear();
    }

    /// L'objet est-il déformé par le squelette (poids présents) ?
    pub fn is_skinned(&self, id: ObjectId) -> bool {
        self.gpu.get(&id).is_some_and(|g| g.skinned)
    }

    /// Envoie (ou réutilise) les tampons d'un objet ; `false` si son maillage
    /// est vide (rien à dessiner).
    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ctx: &MaterialContext,
        draw: &ObjectDraw,
    ) -> bool {
        let fresh = self
            .gpu
            .get(&draw.id)
            .is_none_or(|g| g.mesh_revision != draw.mesh_revision);
        if fresh {
            let Some(mesh) = MeshBuffers::upload(device, queue, &draw.mesh) else {
                self.gpu.remove(&draw.id);
                return false;
            };
            let materials = Materials::new(device, queue, ctx, &draw.mesh);
            self.gpu.insert(
                draw.id,
                ObjectGpu {
                    mesh,
                    materials,
                    mesh_revision: draw.mesh_revision,
                    skin_revision: 0,
                    skinned: false,
                },
            );
        }
        let gpu = self.gpu.get_mut(&draw.id).expect("objet présent");
        // Poids d'un objet déformé : seulement s'ils existent et ont changé.
        match &draw.skin {
            Some(s) if !s.skin.weights.is_empty() => {
                if !gpu.skinned || gpu.skin_revision != s.revision {
                    gpu.mesh.upload_weights(device, queue, &s.skin);
                    gpu.skin_revision = s.revision;
                    gpu.skinned = true;
                }
            }
            // Rigide (ou poids absents) : placement par la matrice du modèle.
            _ => gpu.skinned = false,
        }
        true
    }
}

impl Renderer {
    /// Dessine les objets de l'image dans la passe de scène (après le corps).
    pub(crate) fn draw_objects(&self, pass: &mut wgpu::RenderPass) {
        let Some(ctx) = &self.material_context else { return };
        for (id, offset) in &self.objects.order {
            let Some(gpu) = self.objects.gpu.get(id) else {
                continue;
            };
            pass.set_bind_group(0, &self.bind_group, &[*offset]);
            let mesh = &gpu.mesh;
            if gpu.skinned
                && let Some(weights) = &mesh.weights
            {
                pass.set_pipeline(&self.pipelines.mesh);
                pass.set_vertex_buffer(0, mesh.positions.slice(..));
                pass.set_vertex_buffer(1, mesh.normals.slice(..));
                pass.set_vertex_buffer(2, weights.slice(..));
                pass.set_vertex_buffer(3, mesh.uv_source().slice(..));
            } else {
                pass.set_pipeline(&self.pipelines.object);
                pass.set_vertex_buffer(0, mesh.positions.slice(..));
                pass.set_vertex_buffer(1, mesh.normals.slice(..));
                pass.set_vertex_buffer(2, mesh.uv_source().slice(..));
            }
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            for (range, material) in &gpu.materials.parts {
                pass.set_bind_group(1, gpu.materials.group(ctx, *material), &[]);
                pass.draw_indexed(range.clone(), 0, 0..1);
            }
        }
    }
}
