//! Poids de skinning par diffusion de chaleur (« Bone Heat »), d'après
//! Baran & Popović, *Automatic Rigging and Animation of 3D Characters*,
//! SIGGRAPH 2007 — l'algorithme des poids automatiques de Blender.
//!
//! Pour chaque os i on résout −Δwᵢ + H·wᵢ = H·pᵢ sur la surface, où Δ est le
//! Laplacien cotangent, H_jj = c·k/d(j)² (d : distance à l'os le plus proche,
//! k : nombre d'os à égalité, c : `HEAT_STRENGTH`) et pᵢⱼ = 1/k si i fait
//! partie des plus proches.
//!
//! Mise en œuvre : le système est multiplié par la matrice de masse pour être
//! symétrique défini positif, puis résolu par gradient conjugué préconditionné
//! (Jacobi), un os par tâche rayon. Les cotangentes négatives sont tronquées
//! à 0 pour garantir le principe du maximum (poids dans [0, 1]).
//!
//! Comme dans l'article et dans Blender, un os le plus proche ne chauffe le
//! sommet que s'il est « visible » (le segment sommet–os ne traverse pas la
//! surface) ; sinon k = 0 et le sommet ne reçoit ses poids que par diffusion.

use rayon::prelude::*;

use super::sparse::{conjugate_gradient, cotangent_laplacian};
use super::visibility::nearest_visible;
use crate::{BoneId, Mesh, Skeleton, Skin, VertexWeights};

/// Poids en dessous duquel une influence est ignorée.
const MIN_WEIGHT: f64 = 1e-3;
/// Constante c de H = c/d² : 1 dans l'article ; 4 divise par deux la portée
/// de la diffusion, ce qui évite que les os des membres débordent largement
/// sur un tronc épais (flancs, ventre) sans durcir les articulations.
const HEAT_STRENGTH: f64 = 4.0;

pub fn heat_weights(mesh: &Mesh, skeleton: &Skeleton) -> Skin {
    let n = mesh.positions.len();
    if skeleton.is_empty() || mesh.indices.is_empty() {
        return Skin::automatic(mesh, skeleton);
    }
    if let Some(skin) = welded_heat_weights(mesh, skeleton) {
        return skin;
    }
    let (segments, ids) = super::deform_segments(skeleton);
    if segments.is_empty() {
        return Skin::automatic(mesh, skeleton);
    }

    // Os les plus proches de chaque sommet (égalités à 0,1 % près) qu'il voit.
    let nearest = nearest_visible(mesh, &segments);

    let (laplacian, stiffness_diag, mass) = cotangent_laplacian(mesh);
    // Diagonale du système (L + M·H) et coefficient M·H de chaque sommet.
    let heat: Vec<f64> = nearest
        .iter()
        .zip(&mass)
        .map(|(near, m)| {
            HEAT_STRENGTH * m * near.ties.len() as f64 / (near.distance as f64 * near.distance as f64)
        })
        .collect();
    let diag: Vec<f64> = stiffness_diag
        .iter()
        .zip(&heat)
        .map(|(l, h)| (l + h).max(1e-20))
        .collect();

    // Une résolution par os, en parallèle ; seules les influences utiles
    // sont gardées (liste creuse) pour limiter la mémoire.
    let per_bone: Vec<Vec<(u32, f32)>> = (0..segments.len())
        .into_par_iter()
        .map(|bone| {
            let b: Vec<f64> = nearest
                .iter()
                .zip(&heat)
                .map(|(near, h)| {
                    if near.ties.contains(&(bone as u16)) {
                        h / near.ties.len() as f64
                    } else {
                        0.0
                    }
                })
                .collect();
            if b.iter().all(|v| *v == 0.0) {
                return Vec::new();
            }
            // Point de départ : affectation au plus proche.
            let mut x: Vec<f64> = nearest
                .iter()
                .map(|near| {
                    if near.ties.contains(&(bone as u16)) {
                        1.0 / near.ties.len() as f64
                    } else {
                        0.0
                    }
                })
                .collect();
            conjugate_gradient(&laplacian, &diag, &b, &mut x);
            x.iter()
                .enumerate()
                .filter(|(_, w)| **w > MIN_WEIGHT)
                .map(|(v, w)| (v as u32, *w as f32))
                .collect()
        })
        .collect();

    let mut influences: Vec<Vec<(BoneId, f32)>> = vec![Vec::new(); n];
    for (bone, list) in per_bone.into_iter().enumerate() {
        for (v, w) in list {
            influences[v as usize].push((ids[bone], w));
        }
    }
    let weights = influences
        .into_par_iter()
        .enumerate()
        .map(|(v, inf)| {
            if inf.is_empty() {
                // Sommet isolé (sans triangle) ou partie qui ne voit aucun os
                // (œil dans son orbite) : os le plus proche.
                VertexWeights::single(ids[nearest[v].closest as usize])
            } else {
                VertexWeights::from_influences(inf)
            }
        })
        .collect();
    Skin { weights }
}

/// Les sommets dupliqués (coutures UV, arêtes vives) sont soudés avant le
/// calcul, puis reçoivent tous les poids de leur représentant : la peau ne
/// se fend pas aux coutures. `None` si le maillage n'a pas de doublons.
fn welded_heat_weights(mesh: &Mesh, skeleton: &Skeleton) -> Option<Skin> {
    let (map, unique) = mesh.weld_map();
    if unique == mesh.positions.len() {
        return None;
    }
    let mut compact = vec![u32::MAX; map.len()];
    let mut positions = Vec::with_capacity(unique);
    for (i, rep) in map.iter().enumerate() {
        if *rep as usize == i {
            compact[i] = positions.len() as u32;
            positions.push(mesh.positions[i]);
        }
    }
    let welded = Mesh {
        positions,
        indices: mesh
            .indices
            .iter()
            .map(|i| compact[map[*i as usize] as usize])
            .collect(),
        ..Default::default()
    };
    let skin = heat_weights(&welded, skeleton);
    Some(Skin {
        weights: map
            .iter()
            .map(|rep| skin.weights[compact[*rep as usize] as usize])
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn heat_weights_are_smooth_and_partitioned() {
        let mesh = Mesh::capsule(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.2, 16, 16);
        let mut s = Skeleton::new();
        let a = s.add_bone_world("a", None, Vec3::ZERO, Vec3::Y);
        s.add_bone_world("b", Some(a), Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
        let skin = heat_weights(&mesh, &s);
        for (p, vw) in mesh.positions.iter().zip(&skin.weights) {
            let total: f32 = vw.iter().map(|(_, w)| w).sum();
            assert!((total - 1.0).abs() < 1e-3);
            if p.y < 0.4 {
                assert!(vw.weight_of(BoneId(0)) > 0.95, "{p:?} {vw:?}");
            }
            if p.y > 1.6 {
                assert!(vw.weight_of(BoneId(1)) > 0.95, "{p:?} {vw:?}");
            }
        }
        // Transition douce autour de l'articulation (y = 1).
        let joint = mesh
            .positions
            .iter()
            .position(|p| (p.y - 1.0).abs() < 0.07)
            .unwrap();
        let w = skin.weights[joint].weight_of(BoneId(0));
        assert!(w > 0.2 && w < 0.8, "{w}");
    }
}
