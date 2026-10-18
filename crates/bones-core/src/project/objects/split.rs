//! Découpe des objets pour l'écriture : pool de textures partagé, et
//! extraction des données lourdes de chaque objet (le document ne garde que
//! les matériaux et les parties, comme pour le corps).

use std::sync::Arc;

use super::super::document::TEXTURES_DIR;
use super::ObjectCounts;
use crate::scene::{Binding, SceneObject};
use crate::{Material, Mesh, Skin, Texture};

/// Données lourdes d'un objet, à écrire telles quelles (partagées par `Arc`).
pub(crate) struct ObjectHeavy {
    pub mesh: Arc<Mesh>,
    /// Poids d'un objet déformé.
    pub skin: Option<Arc<Skin>>,
}

/// Objets allégés (pour le document JSON) et leurs données lourdes.
pub(crate) struct Stripped {
    pub counts: Vec<ObjectCounts>,
    pub heavy: Vec<ObjectHeavy>,
}

/// Pool de textures partagé : une image (comparée par pointeur) employée par
/// plusieurs matériaux, du corps ou d'objets, n'est écrite qu'une fois.
#[derive(Default)]
pub(crate) struct TexturePool {
    textures: Vec<(String, Texture)>,
}

impl TexturePool {
    /// Matériaux sans les octets de leurs textures et, pour chacun, l'indice
    /// de sa texture de couleur dans le pool.
    pub fn strip(&mut self, materials: &[Material]) -> (Vec<Material>, Vec<Option<u32>>) {
        let mut indices = Vec::with_capacity(materials.len());
        let stripped = materials
            .iter()
            .map(|m| {
                let mut m = m.clone();
                let index = m.base_color_texture.as_mut().map(|t| {
                    let found = self
                        .textures
                        .iter()
                        .position(|(_, u)| Arc::ptr_eq(&u.bytes, &t.bytes));
                    let i = found.unwrap_or_else(|| {
                        let name = format!("{TEXTURES_DIR}{}.{}", self.textures.len(), t.extension());
                        self.textures.push((name, t.clone()));
                        self.textures.len() - 1
                    });
                    t.bytes = Arc::from([]);
                    i as u32
                });
                indices.push(index);
                m
            })
            .collect();
        (stripped, indices)
    }

    /// Textures accumulées, avec leur nom d'entrée.
    pub fn into_entries(self) -> Vec<(String, Texture)> {
        self.textures
    }
}

/// Allège chaque objet (géométrie et poids retirés, textures mises au pool)
/// et retourne leurs données lourdes. `objects` est la copie destinée au
/// document.
pub(crate) fn strip_objects(objects: &mut [SceneObject], pool: &mut TexturePool) -> Stripped {
    let len = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let mut counts = Vec::with_capacity(objects.len());
    let mut heavy = Vec::with_capacity(objects.len());
    for o in objects.iter_mut() {
        let mesh = o.mesh.clone();
        let skin = o.binding.skin().map(|s| s.skin.clone());
        let (materials, material_textures) = pool.strip(&mesh.materials);
        counts.push(ObjectCounts {
            positions: len(mesh.positions.len()),
            normals: len(mesh.normals.len()),
            uvs: len(mesh.uvs.len()),
            indices: len(mesh.indices.len()),
            weights: skin.as_ref().map_or(0, |s| len(s.weights.len())),
            material_textures,
        });
        o.mesh = Arc::new(Mesh {
            materials,
            submeshes: mesh.submeshes.clone(),
            ..Mesh::default()
        });
        if let Binding::Skinned(s) = &mut o.binding {
            s.skin = Arc::new(Skin::default());
        }
        heavy.push(ObjectHeavy { mesh, skin });
    }
    Stripped { counts, heavy }
}
