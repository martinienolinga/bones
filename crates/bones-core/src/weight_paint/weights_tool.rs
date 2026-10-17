//! Outils du menu « Weights » de Blender, appliqués à tout le maillage (une
//! étape d'annulation côté éditeur). Chacun agit sur l'os peint, sauf
//! « Normalize All », « Limit Total » et « Mirror » qui agissent sur tous les
//! os. « Automatic » est traité par l'éditeur (recalcul des poids).

use rayon::prelude::*;

use super::caches::MeshCaches;
use super::influence::{self, Influences};
use super::{WeightPaintSettings, WeightsTool};
use crate::{BoneId, Mesh, Skeleton, Skin};

/// Applique `tool`. Retourne vrai si des poids ont changé. `bone` est l'os
/// peint (requis par les outils qui n'agissent que sur lui).
pub(crate) fn run(
    skin: &mut Skin,
    skeleton: &Skeleton,
    mesh: &Mesh,
    caches: &MeshCaches,
    bone: Option<BoneId>,
    tool: WeightsTool,
    settings: &WeightPaintSettings,
) -> bool {
    let deform = influence::deform_flags(skeleton);
    match tool {
        WeightsTool::NormalizeAll => map_all(skin, |i| influence::normalize_deform(i, &deform)),
        WeightsTool::LimitTotal { max } => map_all(skin, |i| influence::limit(i, max.max(1) as usize, None)),
        WeightsTool::Mirror => mirror_weights(skin, skeleton, mesh, caches),
        WeightsTool::Clean { threshold } => bone.is_some_and(|b| {
            per_bone(skin, b, &deform, settings, false, |w| {
                if w < threshold { 0.0 } else { w }
            })
        }),
        WeightsTool::Invert => bone.is_some_and(|b| per_bone(skin, b, &deform, settings, true, |w| 1.0 - w)),
        WeightsTool::Levels { offset, gain } => {
            bone.is_some_and(|b| per_bone(skin, b, &deform, settings, true, |w| w * gain + offset))
        }
        WeightsTool::Quantize { steps } => {
            let s = steps.max(1) as f32;
            bone.is_some_and(|b| per_bone(skin, b, &deform, settings, false, |w| (w * s).round() / s))
        }
        WeightsTool::Smooth { iterations, factor } => {
            bone.is_some_and(|b| smooth(skin, caches, b, &deform, settings, iterations, factor))
        }
        WeightsTool::Automatic => false,
    }
}

/// Transforme les influences de chaque sommet ; vrai si quelque chose change.
fn map_all(skin: &mut Skin, f: impl Fn(&mut Influences) + Sync) -> bool {
    skin.weights
        .par_iter_mut()
        .map(|vw| {
            let mut infl = influence::decode(vw);
            f(&mut infl);
            let enc = influence::encode(&infl);
            let changed = enc != *vw;
            *vw = enc;
            changed
        })
        .reduce(|| false, |a, b| a || b)
}

/// Transforme le poids de `bone` à chaque sommet (tous si `all`, sinon
/// seulement là où il a déjà du poids), puis renormalise les autres.
fn per_bone(
    skin: &mut Skin,
    bone: BoneId,
    deform: &[bool],
    settings: &WeightPaintSettings,
    all: bool,
    map: impl Fn(f32) -> f32 + Sync,
) -> bool {
    skin.weights
        .par_iter_mut()
        .map(|vw| {
            let cur = vw.weight_of(bone);
            if !all && cur <= 0.0 {
                return false;
            }
            let mut infl = influence::decode(vw);
            influence::set_weight(&mut infl, bone, map(cur).clamp(0.0, 1.0));
            if settings.auto_normalize {
                influence::auto_normalize(&mut infl, bone, deform);
            }
            let enc = influence::encode(&infl);
            let changed = enc != *vw;
            *vw = enc;
            changed
        })
        .reduce(|| false, |a, b| a || b)
}

/// Lisse le poids de `bone` sur les voisins (plusieurs passes), puis écrit en
/// renormalisant les autres os.
fn smooth(
    skin: &mut Skin,
    caches: &MeshCaches,
    bone: BoneId,
    deform: &[bool],
    settings: &WeightPaintSettings,
    iterations: u32,
    factor: f32,
) -> bool {
    let n = skin.weights.len();
    let mut w: Vec<f32> = skin.weights.par_iter().map(|vw| vw.weight_of(bone)).collect();
    let factor = factor.clamp(0.0, 1.0);
    for _ in 0..iterations.min(100) {
        w = (0..n)
            .into_par_iter()
            .map(|v| {
                let nb = caches.neighbors_of(v);
                if nb.is_empty() {
                    w[v]
                } else {
                    let avg = nb.iter().map(|&r| w[r as usize]).sum::<f32>() / nb.len() as f32;
                    w[v] + (avg - w[v]) * factor
                }
            })
            .collect();
    }
    skin.weights
        .par_iter_mut()
        .enumerate()
        .map(|(v, vw)| {
            let mut infl = influence::decode(vw);
            influence::set_weight(&mut infl, bone, w[v]);
            if settings.auto_normalize {
                influence::auto_normalize(&mut infl, bone, deform);
            }
            let enc = influence::encode(&infl);
            let changed = enc != *vw;
            *vw = enc;
            changed
        })
        .reduce(|| false, |a, b| a || b)
}

/// Recopie les poids du côté +X (gauche) sur le côté −X (droite) en
/// échangeant les os `.l` / `.r` : symétrise le maillage.
fn mirror_weights(skin: &mut Skin, skeleton: &Skeleton, mesh: &Mesh, caches: &MeshCaches) -> bool {
    let bone_mirror = crate::rig::mirror::mirror_map(skeleton);
    let updates: Vec<(usize, crate::VertexWeights)> = (0..skin.weights.len())
        .into_par_iter()
        .filter_map(|v| {
            let m = caches.mirror[v] as usize;
            if m == v || mesh.positions.get(v)?.x <= 0.0 {
                return None;
            }
            let mirrored: Influences = influence::decode(&skin.weights[v])
                .iter()
                .map(|(b, w)| (bone_mirror[b.index()], *w))
                .collect();
            Some((m, influence::encode(&mirrored)))
        })
        .collect();
    let mut changed = false;
    for (m, vw) in updates {
        if skin.weights[m] != vw {
            skin.weights[m] = vw;
            changed = true;
        }
    }
    changed
}
