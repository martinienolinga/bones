//! Cadrage des vignettes : quelques centaines de sommets du maillage servent
//! de sondes, déformées (skinning linéaire sur le CPU) à chaque image cuite ;
//! leur étendue dans le plan de la caméra donne un cadrage exact de la
//! silhouette — bien meilleur que la boîte des os, qui ignore l'épaisseur du
//! corps et ne suit pas une vue de trois quarts.

use glam::{Mat4, Vec2, Vec3};

use super::{Shot, StageModel, plane_axes};
use crate::{BoneId, Pose, Rig, VertexWeights};

/// Nombre maximal de sondes par ensemble.
const MAX_PROBES: usize = 900;

/// Poids minimal de la tête et du visage pour qu'un sommet serve au gros plan.
const HEAD_WEIGHT: f32 = 0.5;

/// Sommet sonde : position de repos et influences.
#[derive(Clone, Copy)]
struct Probe {
    position: Vec3,
    weights: VertexWeights,
}

/// Sondes du corps entier et de la tête.
pub(super) struct Probes {
    body: Vec<Probe>,
    head: Vec<Probe>,
}

impl Probes {
    pub fn new(rig: &Rig) -> Self {
        let mesh = &rig.mesh;
        let weights = &rig.skin.weights;
        let all: Vec<Probe> = mesh
            .positions
            .iter()
            .zip(weights)
            .map(|(p, w)| Probe {
                position: *p,
                weights: *w,
            })
            .collect();
        let head_bones = head_bones(rig);
        let head: Vec<Probe> = all
            .iter()
            .filter(|p| head_bones.iter().map(|b| p.weights.weight_of(*b)).sum::<f32>() >= HEAD_WEIGHT)
            .copied()
            .collect();
        Self {
            body: thin(all),
            head: thin(head),
        }
    }

    fn get(&self, shot: Shot) -> &[Probe] {
        match shot {
            Shot::Body => &self.body,
            Shot::Face => &self.head,
        }
    }
}

/// Os de la tête : `head.x` et les os du visage (contrat de l'auto-rig).
fn head_bones(rig: &Rig) -> Vec<BoneId> {
    let mut bones: Vec<BoneId> = crate::autorig::face::present(&rig.skeleton)
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    bones.extend(rig.skeleton.find("head.x"));
    bones
}

/// Garde au plus [`MAX_PROBES`] sondes, réparties régulièrement.
fn thin(probes: Vec<Probe>) -> Vec<Probe> {
    let step = probes.len().div_ceil(MAX_PROBES).max(1);
    probes.into_iter().step_by(step).collect()
}

/// Zone à cadrer dans le plan de la caméra : centre monde et demi-étendues
/// (droite, haut).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub center: Vec3,
    pub half: Vec2,
}

/// Étendue des sondes déformées sur toutes les poses. Sans sonde (modèle
/// sans maillage), on se rabat sur les têtes et queues des os.
pub(super) fn extent(model: &StageModel, poses: &[Pose], shot: Shot) -> Extent {
    let (right, up) = plane_axes(shot);
    let forward = right.cross(up);
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    let mut depth = (0.0, 0usize);
    let mut add = |p: Vec3| {
        let q = Vec2::new(p.dot(right), p.dot(up));
        lo = lo.min(q);
        hi = hi.max(q);
        depth.0 += p.dot(forward);
        depth.1 += 1;
    };
    let probes = model.probes.get(shot);
    let (mut world, mut joints) = (Vec::new(), Vec::new());
    for pose in poses {
        model.skinning(pose, &mut world, &mut joints);
        if probes.is_empty() {
            for (w, b) in world.iter().zip(model.skeleton.bones()) {
                let (h, t) = crate::head_tail(w, b.length);
                add(h);
                add(t);
            }
        }
        for probe in probes {
            add(skin(probe, &joints));
        }
    }
    if depth.1 == 0 {
        return Extent {
            center: Vec3::new(0.0, 1.0, 0.0),
            half: Vec2::ONE,
        };
    }
    let mid = (lo + hi) * 0.5;
    let d = depth.0 / depth.1 as f32;
    Extent {
        center: right * mid.x + up * mid.y + forward * d,
        half: (hi - lo) * 0.5,
    }
}

/// Position déformée d'une sonde (skinning linéaire).
fn skin(probe: &Probe, joints: &[Mat4]) -> Vec3 {
    let mut sum = Vec3::ZERO;
    let mut total = 0.0;
    for (bone, w) in probe.weights.iter() {
        if let Some(m) = joints.get(bone.index()) {
            sum += w * m.transform_point3(probe.position);
            total += w;
        }
    }
    if total < 1e-6 { probe.position } else { sum / total }
}
