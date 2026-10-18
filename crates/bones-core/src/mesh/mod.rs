//! Maillages triangulés, matériaux, textures et primitives de démonstration.

mod material;
mod place;
mod primitives;
mod shapes;
#[cfg(test)]
mod tests;
mod weld;

use std::ops::Range;

use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

pub use material::{Material, Texture, TextureError, sniff_mime};
pub use shapes::Primitive;

/// Portion du maillage (plage d'indices) dessinée avec un matériau.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submesh {
    pub indices: Range<u32>,
    /// Indice dans [`Mesh::materials`] ; `None` : matériau par défaut.
    pub material: Option<usize>,
}

/// Maillage triangulé indexé. Positions, normales et coordonnées de texture
/// sont stockées dans des tableaux séparés, envoyés tels quels au GPU
/// (aucune copie intermédiaire).
///
/// Coordonnées de texture : origine en haut à gauche (convention glTF).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    /// Vide, ou une coordonnée par sommet.
    #[serde(default)]
    pub uvs: Vec<Vec2>,
    #[serde(default)]
    pub materials: Vec<Material>,
    /// Vide : tout le maillage avec le matériau par défaut.
    #[serde(default)]
    pub submeshes: Vec<Submesh>,
}

impl Mesh {
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn has_uvs(&self) -> bool {
        !self.uvs.is_empty() && self.uvs.len() == self.positions.len()
    }

    /// Parties à dessiner, avec leur matériau (au moins une si le maillage
    /// n'est pas vide).
    pub fn parts(&self) -> Vec<Submesh> {
        if self.submeshes.is_empty() {
            vec![Submesh {
                indices: 0..self.indices.len() as u32,
                material: None,
            }]
        } else {
            self.submeshes.clone()
        }
    }

    /// Normales lissées, pondérées par l'aire des triangles.
    pub fn compute_normals(&mut self) {
        let mut normals = vec![Vec3::ZERO; self.positions.len()];
        for tri in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let n = (self.positions[b] - self.positions[a]).cross(self.positions[c] - self.positions[a]);
            normals[a] += n;
            normals[b] += n;
            normals[c] += n;
        }
        for n in &mut normals {
            *n = n.try_normalize().unwrap_or(Vec3::Y);
        }
        self.normals = normals;
    }

    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let first = *self.positions.first()?;
        Some(
            self.positions
                .iter()
                .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p))),
        )
    }

    /// Ajoute un autre maillage (sommets, coordonnées de texture, matériaux
    /// et parties sont fusionnés).
    pub fn append(&mut self, other: &Mesh) {
        let offset = self.positions.len() as u32;
        let index_offset = self.indices.len() as u32;
        let material_offset = self.materials.len();

        // Les parties ne deviennent explicites que si l'un des deux en a besoin.
        let explicit = !self.submeshes.is_empty()
            || !other.submeshes.is_empty()
            || !self.materials.is_empty()
            || !other.materials.is_empty();
        if explicit {
            let mine = self.parts();
            let theirs = other.parts().into_iter().map(|s| Submesh {
                indices: s.indices.start + index_offset..s.indices.end + index_offset,
                material: s.material.map(|m| m + material_offset),
            });
            self.submeshes = mine
                .into_iter()
                .chain(theirs)
                .filter(|s| !s.indices.is_empty())
                .collect();
        }

        if self.has_uvs() || other.has_uvs() {
            self.uvs.resize(self.positions.len(), Vec2::ZERO);
            if other.has_uvs() {
                self.uvs.extend_from_slice(&other.uvs);
            } else {
                self.uvs
                    .resize(self.positions.len() + other.positions.len(), Vec2::ZERO);
            }
        }
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.indices.extend(other.indices.iter().map(|i| i + offset));
        self.materials.extend(other.materials.iter().cloned());
    }

    /// Ajoute une pièce de géométrie dessinée avec un matériau déjà présent
    /// dans [`Mesh::materials`] (les matériaux de `part` sont ignorés).
    pub fn append_part(&mut self, mut part: Mesh, material: Option<usize>) {
        part.submeshes = vec![Submesh {
            indices: 0..part.indices.len() as u32,
            material: material.filter(|m| *m < self.materials.len()),
        }];
        part.materials.clear();
        let materials = std::mem::take(&mut self.materials);
        self.append(&part);
        self.materials = materials;
    }

    /// Libère la capacité excédentaire des tableaux.
    pub fn shrink_to_fit(&mut self) {
        self.positions.shrink_to_fit();
        self.normals.shrink_to_fit();
        self.indices.shrink_to_fit();
        self.uvs.shrink_to_fit();
        self.submeshes.shrink_to_fit();
    }

    /// Applique une rotation puis une translation à tous les sommets.
    pub fn transform(&mut self, rotation: Quat, translation: Vec3) {
        for p in &mut self.positions {
            *p = rotation * *p + translation;
        }
        for n in &mut self.normals {
            *n = rotation * *n;
        }
    }

    /// Applique une échelle uniforme.
    pub fn scale(&mut self, factor: f32) {
        for p in &mut self.positions {
            *p *= factor;
        }
    }
}
