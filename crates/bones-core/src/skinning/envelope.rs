//! Poids par enveloppes : décroissance en fonction de la distance aux os.
//! Instantané, utilisé pour les aperçus et comme repli.

use glam::Vec3;
use rayon::prelude::*;

use crate::{BoneId, Mesh, Skeleton, Skin, VertexWeights, head_tail};

impl Skin {
    /// Poids automatiques : chaque sommet est influencé par les os les plus
    /// proches (distance au segment tête–queue), avec une décroissance en 1/d⁴
    /// relative à l'os le plus proche. Calcul parallélisé sur les sommets.
    pub fn automatic(mesh: &Mesh, skeleton: &Skeleton) -> Self {
        let world = skeleton.rest_world();
        let segments: Vec<(Vec3, Vec3)> = skeleton
            .bones()
            .iter()
            .zip(&world)
            .map(|(b, w)| head_tail(w, b.length))
            .collect();
        if segments.is_empty() {
            return Self {
                weights: vec![VertexWeights::default(); mesh.positions.len()],
            };
        }

        let weights = mesh
            .positions
            .par_iter()
            .map_init(
                || Vec::with_capacity(segments.len()),
                |dists: &mut Vec<f32>, &p| {
                    dists.clear();
                    dists.extend(segments.iter().map(|(a, b)| distance_to_segment(p, *a, *b)));
                    let nearest = dists.iter().copied().fold(f32::INFINITY, f32::min).max(1e-4);
                    let influences = dists
                        .iter()
                        .enumerate()
                        .map(|(i, d)| (BoneId(i as u32), (nearest / d.max(1e-6)).powi(4)))
                        .filter(|(_, w)| *w > 0.01)
                        .collect();
                    VertexWeights::from_influences(influences)
                },
            )
            .collect();
        Self { weights }
    }
}

pub fn distance_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t - p).length()
}
