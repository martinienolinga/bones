//! Ancres et régions du visage, déduites des positions de repos des os du
//! visage et de la surface : os de surface localisés (ancre + rayon),
//! composantes rigides (yeux, dents) et classification haut/bas de la bouche.

use std::collections::HashMap;

use glam::Vec3;

use super::surface::FaceSurface;
use crate::autorig::face::{FaceBone, present};
use crate::{BoneId, Skeleton};

/// Côté d'une lèvre, pour séparer lèvre supérieure et inférieure.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Lip {
    None,
    Upper,
    Lower,
}

/// Os de surface localisé : influence en cloche autour d'une ancre.
pub(super) struct Secondary {
    pub bone: BoneId,
    pub anchor: u32,
    pub radius: f32,
    pub lip: Lip,
}

/// Bouche : centre, rayon d'action et semences des lèvres haute/basse.
pub(super) struct Mouth {
    pub radius: f32,
    pub upper: Vec<u32>,
    pub lower: Vec<u32>,
}

/// Tout ce qu'il faut pour peser le visage.
pub(super) struct FaceRig {
    pub head: BoneId,
    pub jaw: Option<BoneId>,
    pub secondary: Vec<Secondary>,
    /// Composante connexe rigide → os (yeux, dents).
    pub rigid: HashMap<u32, BoneId>,
    /// Os de la langue (tête, queue) : la pièce « langue » les suit au plus proche.
    pub tongue: Vec<(BoneId, Vec3, Vec3)>,
    pub tongue_comp: Option<u32>,
    pub mouth: Option<Mouth>,
}

impl FaceRig {
    pub fn build(skeleton: &Skeleton, surface: &FaceSurface) -> Option<FaceRig> {
        let head = skeleton.find("head.x")?;
        let faces = present(skeleton);
        if faces.is_empty() {
            return None;
        }
        let pos: HashMap<FaceBone, (Vec3, Vec3)> = faces
            .iter()
            .map(|(b, id)| (*b, skeleton.rest_head_tail(*id)))
            .collect();
        let id_of: HashMap<FaceBone, BoneId> = faces.iter().map(|(b, id)| (*b, *id)).collect();

        // Échelle du visage : écart inter-oculaire, sinon longueur de la tête.
        let io = match (pos.get(&FaceBone::EyeL), pos.get(&FaceBone::EyeR)) {
            (Some(l), Some(r)) => l.0.distance(r.0),
            _ => skeleton.bone(head).length.max(1e-2),
        };
        let stats = component_stats(surface);
        let total = surface.len();

        let mut rigid = HashMap::new();
        let mut secondary = Vec::new();
        let mut tongue = Vec::new();
        let mut tongue_comp = None;
        for (bone, (bh, bt)) in &pos {
            let id = id_of[bone];
            match bone {
                FaceBone::Jaw => {}
                FaceBone::EyeL | FaceBone::EyeR => {
                    if let Some(c) = rigid_comp(&stats, total, *bh, 0.6 * io) {
                        rigid.insert(c, id);
                    }
                }
                FaceBone::TeethTop | FaceBone::TeethBottom => {
                    if let Some(c) = rigid_comp(&stats, total, *bt, 0.6 * io) {
                        rigid.insert(c, id);
                    }
                }
                FaceBone::Tongue1 | FaceBone::Tongue2 | FaceBone::Tongue3 => {
                    tongue.push((id, *bh, *bt));
                    tongue_comp =
                        tongue_comp.or_else(|| rigid_comp(&stats, total, (*bh + *bt) * 0.5, 0.6 * io));
                }
                _ => {
                    if let Some((anchor, radius, lip)) = surface_influence(surface, *bone, *bt, bh, io) {
                        secondary.push(Secondary {
                            bone: id,
                            anchor,
                            radius,
                            lip,
                        });
                    }
                }
            }
        }

        let mouth = build_mouth(surface, &secondary, io);
        Some(FaceRig {
            head,
            jaw: skeleton.find("jaw.x"),
            secondary,
            rigid,
            tongue,
            tongue_comp,
            mouth,
        })
    }
}

/// Ancre, rayon et côté lèvre d'un os de surface.
fn surface_influence(
    s: &FaceSurface,
    bone: FaceBone,
    tail: Vec3,
    head: &Vec3,
    io: f32,
) -> Option<(u32, f32, Lip)> {
    use FaceBone::*;
    let anchor = s.nearest(tail)?;
    let eye_len = head.distance(tail).max(2e-3);
    let (radius, lip) = match bone {
        BrowInL | BrowInR | BrowMidL | BrowMidR | BrowOutL | BrowOutR => (0.55 * io, Lip::None),
        CheekL | CheekR => (0.6 * io, Lip::None),
        Nose => (0.5 * io, Lip::None),
        NostrilL | NostrilR => (0.4 * io, Lip::None),
        Chin => (0.6 * io, Lip::None),
        LipTop | LipTopL | LipTopR => (0.5 * io, Lip::Upper),
        LipBottom | LipBottomL | LipBottomR => (0.5 * io, Lip::Lower),
        LipCornerL | LipCornerR => (0.45 * io, Lip::None),
        EyelidTopL | EyelidTopR | EyelidBottomL | EyelidBottomR => (2.2 * eye_len, Lip::None),
        EarL | EarR => (2.5 * eye_len, Lip::None),
        _ => return None,
    };
    Some((anchor, radius, lip))
}

/// Construit la région de la bouche à partir des os de lèvre.
fn build_mouth(s: &FaceSurface, secondary: &[Secondary], io: f32) -> Option<Mouth> {
    let upper: Vec<u32> = secondary
        .iter()
        .filter(|x| x.lip == Lip::Upper)
        .map(|x| x.anchor)
        .collect();
    let lower: Vec<u32> = secondary
        .iter()
        .filter(|x| x.lip == Lip::Lower)
        .map(|x| x.anchor)
        .collect();
    if upper.is_empty() || lower.is_empty() {
        return None;
    }
    let _ = s;
    Some(Mouth {
        radius: 0.9 * io,
        upper,
        lower,
    })
}

/// Petite composante connexe dont le centre est le plus proche de `p` (dans
/// `max_dist`) : le globe oculaire ou les dents, jamais la peau de la tête
/// (grande composante). On compare les centres, car la peau de la paupière
/// peut passer plus près du point que le globe lui-même.
fn rigid_comp(stats: &HashMap<u32, (usize, Vec3)>, total: usize, p: Vec3, max_dist: f32) -> Option<u32> {
    // Le plafond écarte seulement la grande composante (tête + corps) ; un
    // globe oculaire stylisé peut être assez gros.
    stats
        .iter()
        .filter(|(_, (size, _))| *size >= 4 && (*size as f32) < 0.35 * total as f32)
        .map(|(c, (_, center))| (*c, center.distance(p)))
        .filter(|(_, d)| *d < max_dist)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(c, _)| c)
}

/// Nombre de sommets et centre de chaque composante.
fn component_stats(s: &FaceSurface) -> HashMap<u32, (usize, Vec3)> {
    let mut acc: HashMap<u32, (usize, Vec3)> = HashMap::new();
    for v in 0..s.len() as u32 {
        let e = acc.entry(s.component(v)).or_insert((0, Vec3::ZERO));
        e.0 += 1;
        e.1 += s.positions[v as usize];
    }
    for (count, sum) in acc.values_mut() {
        *sum /= *count as f32;
    }
    acc
}
