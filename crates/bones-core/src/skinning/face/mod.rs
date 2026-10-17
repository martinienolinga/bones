//! Poids du visage, façon Auto-Rig Pro / Rigify.
//!
//! La diffusion de chaleur seule (Baran & Popović 2007) place mal les os du
//! visage : tout proches les uns des autres, ils fuient d'une lèvre à l'autre
//! ou d'une paupière à l'œil. On combine donc :
//!
//! - une **base** par diffusion de chaleur sur le corps + tête + mâchoire (les
//!   os de surface du visage en sont exclus) ;
//! - des **influences localisées** des os de surface (sourcils, joues, nez,
//!   narines, lèvres, commissures, menton, paupières, oreilles), en cloche
//!   autour d'une ancre, selon la **distance géodésique sur la surface** —
//!   même principe que le *geodesic voxel binding* de Dionne & de Lasa 2013,
//!   calculé ici par un Dijkstra sur les arêtes — mélangées en partition de
//!   l'unité avec la base ;
//! - une **séparation haut/bas de la bouche** par proximité géodésique aux
//!   lèvres supérieure et inférieure (la fente de la bouche sépare
//!   naturellement les deux), la partie basse suivant la mâchoire : la bouche
//!   s'ouvre proprement ;
//! - des **yeux rigides** : les globes (pièces séparées) sont à 100 % sur leur
//!   os, sans fuite sur la paupière ni le corps.
//!
//! Les influences localisées étant bornées géodésiquement à de petites zones
//! du visage, rien ne déborde sur le cou ni le corps.

mod anchors;
mod surface;
#[cfg(test)]
mod tests;

use rayon::prelude::*;

use anchors::{FaceRig, Lip};
use surface::{FaceSurface, UNREACHED};

use crate::{BindMethod, BoneId, Mesh, Skeleton, Skin, VertexWeights};

/// Liaison avec rig facial : `base_method` sert pour le corps et la tête.
/// Sans os de visage, c'est la liaison ordinaire.
pub(crate) fn bind_with_face(mesh: &Mesh, skeleton: &Skeleton, base_method: BindMethod) -> Skin {
    let surface = FaceSurface::new(mesh);
    let Some(rig) = FaceRig::build(skeleton, &surface) else {
        return base_method.compute_body(mesh, skeleton);
    };

    // Base : corps + tête + mâchoire, les os de surface du visage exclus.
    let mut base_skel = skeleton.clone();
    for (bone, id) in crate::autorig::face::present(skeleton) {
        if bone != crate::autorig::face::FaceBone::Jaw {
            base_skel.bone_mut(id).deform = false;
        }
    }
    let base = base_method.compute_body(mesh, &base_skel);

    // Champs géodésiques de la bouche (haut / bas) et superposition localisée.
    let mouth = rig.mouth.as_ref().map(|m| {
        (
            surface.geodesic(&m.upper, m.radius),
            surface.geodesic(&m.lower, m.radius),
            m.radius,
        )
    });
    let overlay = overlay_fields(&surface, &rig, mouth.as_ref());

    let weights = (0..mesh.vertex_count())
        .into_par_iter()
        .map(|v| vertex_weights(v, &surface, &rig, &base, &overlay, mouth.as_ref()))
        .collect();
    Skin { weights }
}

/// Influences de chaque os de surface, par sommet soudé, déjà séparées
/// haut/bas pour les lèvres. Calcul parallèle sur les os.
fn overlay_fields(
    surface: &FaceSurface,
    rig: &FaceRig,
    mouth: Option<&(Vec<f32>, Vec<f32>, f32)>,
) -> Vec<Vec<(BoneId, f32)>> {
    let contributions: Vec<Vec<(u32, BoneId, f32)>> = rig
        .secondary
        .par_iter()
        .map(|s| {
            let dist = surface.geodesic(&[s.anchor], s.radius);
            dist.iter()
                .enumerate()
                .filter(|(_, d)| **d < UNREACHED)
                .filter_map(|(w, d)| {
                    let phi = falloff(*d / s.radius);
                    if phi <= 0.0 || !lip_side_ok(s.lip, w, mouth) {
                        return None;
                    }
                    Some((w as u32, s.bone, phi))
                })
                .collect()
        })
        .collect();

    let mut fields = vec![Vec::new(); surface.len()];
    for list in contributions {
        for (w, bone, phi) in list {
            fields[w as usize].push((bone, phi));
        }
    }
    fields
}

/// Un os de lèvre n'agit que du bon côté de la fente.
fn lip_side_ok(lip: Lip, w: usize, mouth: Option<&(Vec<f32>, Vec<f32>, f32)>) -> bool {
    match (lip, mouth) {
        (Lip::None, _) | (_, None) => true,
        (side, Some((up, low, _))) => {
            let (du, dl) = (up[w], low[w]);
            if du == UNREACHED && dl == UNREACHED {
                return false;
            }
            (side == Lip::Upper) == (du <= dl)
        }
    }
}

/// Poids final d'un sommet d'origine.
fn vertex_weights(
    v: usize,
    surface: &FaceSurface,
    rig: &FaceRig,
    base: &Skin,
    overlay: &[Vec<(BoneId, f32)>],
    mouth: Option<&(Vec<f32>, Vec<f32>, f32)>,
) -> VertexWeights {
    let w = surface.welded[v] as usize;
    let comp = surface.component(w as u32);

    // Pièces rigides : yeux et dents.
    if let Some(bone) = rig.rigid.get(&comp) {
        return VertexWeights::single(*bone);
    }
    // Langue : os le plus proche de la chaîne.
    if rig.tongue_comp == Some(comp)
        && let Some((bone, _, _)) = rig.tongue.iter().min_by(|a, b| {
            let d = |x: &(BoneId, glam::Vec3, glam::Vec3)| {
                super::distance_to_segment(surface.positions[w], x.1, x.2)
            };
            d(a).total_cmp(&d(b))
        })
    {
        return VertexWeights::single(*bone);
    }

    let mut infl: Vec<(BoneId, f32)> = base.weights[v].iter().collect();

    // Bouche : la zone proche des lèvres bascule vers la tête (haut) ou la
    // mâchoire (bas), d'autant plus qu'on approche des lèvres.
    if let Some((up, low, radius)) = mouth {
        let (du, dl) = (up[w], low[w]);
        let near = du.min(dl);
        if near < *radius {
            let upper = du <= dl;
            let target = if upper {
                rig.head
            } else {
                rig.jaw.unwrap_or(rig.head)
            };
            let wr = falloff(near / radius);
            for e in &mut infl {
                e.1 *= 1.0 - wr;
            }
            accumulate(&mut infl, target, wr);
        }
    }

    // Os de surface, en partition de l'unité avec la base.
    let sec = &overlay[w];
    let sum: f32 = sec.iter().map(|(_, p)| *p).sum();
    if sum > 1e-4 {
        let alpha = sum.min(1.0);
        for e in &mut infl {
            e.1 *= 1.0 - alpha;
        }
        for (b, p) in sec {
            accumulate(&mut infl, *b, alpha * p / sum);
        }
    }
    VertexWeights::from_influences(infl)
}

fn accumulate(list: &mut Vec<(BoneId, f32)>, bone: BoneId, w: f32) {
    match list.iter_mut().find(|(b, _)| *b == bone) {
        Some(e) => e.1 += w,
        None => list.push((bone, w)),
    }
}

/// Cloche lisse décroissante : 1 à l'ancre, 0 au-delà du rayon (complément
/// du smoothstep).
fn falloff(x: f32) -> f32 {
    if x >= 1.0 {
        0.0
    } else {
        let s = x * x * (3.0 - 2.0 * x);
        1.0 - s
    }
}
