//! Placement d'un maillage : transformation complète (échelle non uniforme
//! et miroir compris) et ménage des matériaux inutilisés.

use glam::{Mat3, Vec3};

use crate::{Mesh, Submesh, Transform};

impl Mesh {
    /// Applique une transformation (translation, rotation, échelle
    /// quelconque). Les normales suivent l'inverse transposée ; un miroir
    /// inverse l'ordre des sommets des triangles.
    pub fn transform_by(&mut self, t: &Transform) {
        if *t == Transform::IDENTITY {
            return;
        }
        let m = t.to_matrix();
        for p in &mut self.positions {
            *p = m.transform_point3(*p);
        }
        let nm = Mat3::from_mat4(m).inverse().transpose();
        if nm.is_finite() {
            for n in &mut self.normals {
                *n = (nm * *n).normalize_or(Vec3::Y);
            }
        }
        if m.determinant() < 0.0 {
            for tri in self.indices.as_chunks_mut::<3>().0 {
                tri.swap(1, 2);
            }
        }
    }

    /// Ne garde que les matériaux utilisés par une partie (indices remis
    /// à jour) : un objet importé ne traîne pas tous ceux du fichier.
    pub fn compact_materials(&mut self) {
        let mut used: Vec<usize> = self.submeshes.iter().filter_map(|s| s.material).collect();
        used.sort_unstable();
        used.dedup();
        let materials = std::mem::take(&mut self.materials);
        self.materials = used.iter().filter_map(|i| materials.get(*i).cloned()).collect();
        for part in &mut self.submeshes {
            part.material = part.material.and_then(|m| used.iter().position(|u| *u == m));
        }
        if self.materials.is_empty() {
            self.submeshes.retain(|s: &Submesh| !s.indices.is_empty());
            if self.submeshes.len() <= 1 {
                self.submeshes.clear();
            }
        }
    }
}
