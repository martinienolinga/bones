//! Transfert de poids d'un maillage à un autre (vêtements, accessoires
//! déformés) : chaque sommet reçoit les poids interpolés du point le plus
//! proche de la surface source, comme le « Data Transfer » de Blender en
//! mode « Nearest Face Interpolated ». Calcul parallèle sur les sommets.

use glam::Vec3;
use rayon::prelude::*;

use super::closest::ClosestTriangles;
use super::{MAX_INFLUENCES, Skin, VertexWeights};
use crate::{BoneId, Mesh};

impl Skin {
    /// Poids de chaque point de `targets` (même espace que `source`),
    /// interpolés depuis `source_skin` au point le plus proche de `source`.
    /// Sans triangle source, les poids sont vides.
    pub fn transfer(source: &Mesh, source_skin: &Skin, targets: &[Vec3]) -> Skin {
        let tree = ClosestTriangles::new(&source.positions, &source.indices);
        let weights = targets
            .par_iter()
            .map(|p| {
                let Some(hit) = tree.closest(*p) else {
                    return VertexWeights::default();
                };
                let mut influences: Vec<(BoneId, f32)> = Vec::with_capacity(3 * MAX_INFLUENCES);
                for (corner, factor) in hit.triangle.iter().zip(hit.bary.to_array()) {
                    let Some(w) = source_skin.weights.get(*corner as usize) else {
                        continue;
                    };
                    for (bone, weight) in w.iter() {
                        match influences.iter_mut().find(|(b, _)| *b == bone) {
                            Some(e) => e.1 += weight * factor,
                            None => influences.push((bone, weight * factor)),
                        }
                    }
                }
                VertexWeights::from_influences(influences)
            })
            .collect();
        Skin { weights }
    }
}
