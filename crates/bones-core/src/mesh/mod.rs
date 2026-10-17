//! Maillages triangulés et primitives de démonstration.

mod primitives;
#[cfg(test)]
mod tests;

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Maillage triangulé indexé. Positions et normales sont stockées dans des
/// tableaux séparés, envoyés tels quels au GPU (aucune copie intermédiaire).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Normales lissées, pondérées par l'aire des triangles.
    pub fn compute_normals(&mut self) {
        let mut normals = vec![Vec3::ZERO; self.positions.len()];
        for tri in self.indices.chunks_exact(3) {
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

    pub fn append(&mut self, other: &Mesh) {
        let offset = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.indices.extend(other.indices.iter().map(|i| i + offset));
    }

    /// Libère la capacité excédentaire des tableaux.
    pub fn shrink_to_fit(&mut self) {
        self.positions.shrink_to_fit();
        self.normals.shrink_to_fit();
        self.indices.shrink_to_fit();
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
}
