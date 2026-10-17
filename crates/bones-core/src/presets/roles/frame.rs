//! Repère du corps : avant, haut, gauche, sol et taille de référence.
//!
//! Les générateurs et le reciblage travaillent dans un repère canonique
//! (X à gauche, Y en haut, Z vers l'avant, comme les personnages de
//! l'application) ; [`BodyFrame::basis`] le ramène dans le monde du squelette.

use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::{Chain, Lr, RoleMap};
use crate::{Skeleton, head_tail};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyFrame {
    /// Direction du regard (horizontale, unitaire).
    pub forward: Vec3,
    /// Verticale (unitaire).
    pub up: Vec3,
    /// Hauteur du sol au repos.
    pub ground: f32,
    /// Taille de référence : hauteur du bassin au-dessus du sol (ou du corps
    /// entier sans bassin).
    pub size: f32,
}

impl Default for BodyFrame {
    fn default() -> Self {
        Self {
            forward: Vec3::Z,
            up: Vec3::Y,
            ground: 0.0,
            size: 1.0,
        }
    }
}

impl BodyFrame {
    /// Repère par défaut d'un squelette : face à +Z, sol au point le plus bas.
    pub fn from_skeleton(skeleton: &Skeleton) -> Self {
        let (lo, hi) = extent(skeleton);
        Self {
            ground: lo.y,
            size: (hi - lo).max_element().max(1e-3),
            ..Self::default()
        }
    }

    pub fn left(&self) -> Vec3 {
        self.up.cross(self.forward).normalize_or(Vec3::X)
    }

    /// Rotation canonique (X gauche, Y haut, Z avant) → monde.
    pub fn basis(&self) -> Quat {
        let forward = (self.forward - self.up * self.forward.dot(self.up)).normalize_or(Vec3::Z);
        let up = self.up.normalize_or(Vec3::Y);
        Quat::from_mat3(&Mat3::from_cols(
            up.cross(forward).normalize_or(Vec3::X),
            up,
            forward,
        ))
        .normalize()
    }

    pub fn to_world(&self, v: Vec3) -> Vec3 {
        self.basis() * v
    }

    pub fn to_canon(&self, v: Vec3) -> Vec3 {
        self.basis().inverse() * v
    }
}

/// Boîte englobante des têtes et queues au repos.
pub(super) fn extent(skeleton: &Skeleton) -> (Vec3, Vec3) {
    let world = skeleton.rest_world();
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for (w, b) in world.iter().zip(skeleton.bones()) {
        let (h, t) = head_tail(w, b.length);
        lo = lo.min(h.min(t));
        hi = hi.max(h.max(t));
    }
    if lo.x > hi.x {
        (Vec3::ZERO, Vec3::ZERO)
    } else {
        (lo, hi)
    }
}

/// Repère estimé d'après les rôles : côtés gauche/droite, position de la
/// tête, direction des pieds.
pub(super) fn estimate(skeleton: &Skeleton, map: &RoleMap) -> BodyFrame {
    let mut frame = BodyFrame::from_skeleton(skeleton);
    let world = skeleton.rest_world();
    let head_of = |b: crate::BoneId| world[b.index()].translation;
    let tail_of = |b: crate::BoneId| head_tail(&world[b.index()], skeleton.bone(b).length).1;
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);

    // Côtés : moyenne des membres gauches moins celle des droits.
    let mut lateral = Vec3::ZERO;
    for chain in map.chains() {
        if let (Some(side), Some(first)) = (chain.side(), map.chain(chain).first()) {
            lateral += head_of(*first) * side.sign();
        }
    }
    let from_sides = flat(lateral).try_normalize().map(|l| l.cross(Vec3::Y));

    // Corps horizontal (animal) : la tête est devant le bassin.
    let root = map.root().or_else(|| map.chain(Chain::Spine).first().copied());
    let head = map
        .chain(Chain::Head)
        .first()
        .or(map.chain(Chain::Neck).last())
        .or(map.chain(Chain::Spine).last())
        .copied();
    let from_head = match (root, head) {
        (Some(r), Some(h)) if r != h => {
            let d = head_of(h) - head_of(r);
            let horizontal = flat(d).length();
            (horizontal > 0.5 * d.length()).then(|| flat(d).normalize())
        }
        _ => None,
    };

    // Pieds : les orteils pointent vers l'avant.
    let mut toes = Vec3::ZERO;
    for side in Lr::BOTH {
        if let Some(foot) = map.bone(Chain::Leg(side), 2) {
            toes += flat(tail_of(foot) - head_of(foot));
        }
    }
    let from_toes = toes.try_normalize();

    frame.forward = match (from_head, from_sides, from_toes) {
        (Some(h), _, _) => h,
        (None, Some(s), Some(t)) if s.dot(t) < -0.3 => t,
        (None, Some(s), _) => s,
        (None, None, Some(t)) => t,
        _ => Vec3::Z,
    };
    if let Some(r) = map.root() {
        let height = head_of(r).y - frame.ground;
        if height > 0.05 * frame.size {
            frame.size = height;
        }
    }
    frame
}
