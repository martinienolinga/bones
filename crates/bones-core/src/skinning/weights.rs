//! Poids de skinning : format compact des influences par sommet et
//! opérations sur l'ensemble des poids.

use bytemuck::{Pod, Zeroable};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{BoneId, BoneRemap};

pub const MAX_INFLUENCES: usize = 4;
const WEIGHT_ONE: u16 = u16::MAX;

/// Influences d'au plus quatre os sur un sommet.
///
/// Format compact (16 octets) directement utilisable comme tampon de
/// sommets GPU : indices en `u16`, poids normalisés en `u16` (65535 = 1.0).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct VertexWeights {
    pub bones: [u16; MAX_INFLUENCES],
    pub weights: [u16; MAX_INFLUENCES],
}

impl VertexWeights {
    pub fn single(bone: BoneId) -> Self {
        Self {
            bones: [bone.0 as u16, 0, 0, 0],
            weights: [WEIGHT_ONE, 0, 0, 0],
        }
    }

    pub fn weight_of(&self, bone: BoneId) -> f32 {
        self.iter().filter(|(b, _)| *b == bone).map(|(_, w)| w).sum()
    }

    /// Influences non nulles, poids en flottants.
    pub fn iter(&self) -> impl Iterator<Item = (BoneId, f32)> + '_ {
        self.bones
            .iter()
            .zip(&self.weights)
            .filter(|(_, w)| **w > 0)
            .map(|(b, w)| (BoneId(*b as u32), *w as f32 / WEIGHT_ONE as f32))
    }

    /// Construit les influences à partir d'une liste quelconque : garde les
    /// quatre plus fortes, normalise et quantifie (la somme vaut exactement 1).
    pub fn from_influences(mut influences: Vec<(BoneId, f32)>) -> Self {
        influences.retain(|(_, w)| *w > 0.0 && w.is_finite());
        influences.sort_by(|a, b| b.1.total_cmp(&a.1));
        influences.truncate(MAX_INFLUENCES);
        let total: f32 = influences.iter().map(|(_, w)| w).sum();
        let mut out = Self::default();
        if total <= 0.0 {
            return out;
        }
        let mut sum: u32 = 0;
        for (i, (b, w)) in influences.iter().enumerate() {
            out.bones[i] = b.0 as u16;
            out.weights[i] = (w / total * WEIGHT_ONE as f32).round() as u16;
            sum += out.weights[i] as u32;
        }
        // L'erreur d'arrondi va à l'influence principale.
        let fixed = out.weights[0] as i64 + WEIGHT_ONE as i64 - sum as i64;
        out.weights[0] = fixed.clamp(0, WEIGHT_ONE as i64) as u16;
        out
    }
}

/// Poids de skinning d'un maillage. La liaison se fait toujours sur la pose de
/// repos courante du squelette : en pose de repos, le maillage n'est pas déformé.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skin {
    pub weights: Vec<VertexWeights>,
}

impl Skin {
    /// Masse relative de chaque os : somme des poids qu'il porte.
    pub fn bone_masses(&self, bone_count: usize) -> Vec<f32> {
        self.weights
            .par_iter()
            .fold(
                || vec![0.0f32; bone_count],
                |mut acc, vw| {
                    for (b, w) in vw.iter() {
                        if let Some(m) = acc.get_mut(b.index()) {
                            *m += w;
                        }
                    }
                    acc
                },
            )
            .reduce(
                || vec![0.0f32; bone_count],
                |mut a, b| {
                    a.iter_mut().zip(b).for_each(|(x, y)| *x += y);
                    a
                },
            )
    }

    /// Met à jour les poids après la suppression d'un os : ses influences
    /// passent à son parent.
    pub fn remap(&mut self, remap: &BoneRemap) {
        self.weights.par_iter_mut().for_each(|vw| {
            let mut influences: Vec<(BoneId, f32)> = Vec::with_capacity(MAX_INFLUENCES);
            for (b, w) in vw.iter() {
                let target = match remap.map.get(b.index()) {
                    Some(Some(new)) => Some(*new),
                    Some(None) => remap.removed_parent,
                    None => None,
                };
                let Some(t) = target else { continue };
                match influences.iter_mut().find(|(x, _)| *x == t) {
                    Some(e) => e.1 += w,
                    None => influences.push((t, w)),
                }
            }
            *vw = VertexWeights::from_influences(influences);
        });
    }
}
