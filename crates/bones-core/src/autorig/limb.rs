//! Membres IK/FK à deux segments (bras, jambes).

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::ik::solve_two_bone;
use crate::{BoneId, BoneRemap, Pose, Skeleton, Transform, head_tail};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimbMode {
    Fk,
    #[default]
    Ik,
}

/// Membre à deux segments (bras ou jambe) avec bascule IK/FK, comme dans
/// Auto-Rig Pro. La pose reste la seule source de vérité : la cible IK et le
/// pôle sont des poignées de manipulation, recalées sur la pose quand on
/// change d'image.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Limb {
    pub name: String,
    pub upper: BoneId,
    pub lower: BoneId,
    /// Main ou pied : garde son orientation monde pendant l'IK.
    pub end: Option<BoneId>,
    pub mode: LimbMode,
    pub target: Vec3,
    pub pole: Vec3,
    #[serde(skip)]
    end_rotation: Option<Quat>,
}

impl Limb {
    pub fn new(name: impl Into<String>, upper: BoneId, lower: BoneId, end: Option<BoneId>) -> Self {
        Self {
            name: name.into(),
            upper,
            lower,
            end,
            mode: LimbMode::Ik,
            target: Vec3::ZERO,
            pole: Vec3::ZERO,
            end_rotation: None,
        }
    }

    pub fn contains(&self, bone: BoneId) -> bool {
        bone == self.upper || bone == self.lower || Some(bone) == self.end
    }

    pub fn is_valid(&self, skeleton: &Skeleton) -> bool {
        skeleton.contains(self.upper)
            && skeleton.contains(self.lower)
            && skeleton.bone(self.lower).parent() == Some(self.upper)
            && self
                .end
                .is_none_or(|e| skeleton.contains(e) && skeleton.bone(e).parent() == Some(self.lower))
    }

    /// Recale la cible et le pôle sur la pose courante (« snap IK → FK »).
    pub fn snap_to_pose(&mut self, skeleton: &Skeleton, world: &[Transform]) {
        let a = world[self.upper.index()].translation;
        let b = world[self.lower.index()].translation;
        let c = head_tail(&world[self.lower.index()], skeleton.bone(self.lower).length).1;
        self.target = c;
        let mid = (a + c) * 0.5;
        let reach = (c - a).length().max(1e-3);
        let out = (b - mid)
            .try_normalize()
            .unwrap_or_else(|| (c - a).any_orthonormal_vector());
        self.pole = b + out * reach;
        self.end_rotation = self.end.map(|e| world[e.index()].rotation);
    }

    /// Applique l'IK si le membre est en mode IK.
    pub fn solve(&self, skeleton: &Skeleton, pose: &mut Pose) {
        if self.mode != LimbMode::Ik || !self.is_valid(skeleton) {
            return;
        }
        solve_two_bone(skeleton, pose, self.upper, self.lower, self.target, self.pole);
        if let (Some(end), Some(rotation)) = (self.end, self.end_rotation) {
            let world = pose.world(skeleton);
            let current = world[end.index()].rotation;
            pose.rotate_world(
                skeleton,
                end,
                &world[self.lower.index()],
                rotation * current.inverse(),
            );
        }
    }

    pub fn remap(&self, remap: &BoneRemap) -> Option<Limb> {
        Some(Limb {
            upper: remap.get(self.upper)?,
            lower: remap.get(self.lower)?,
            end: match self.end {
                Some(e) => Some(remap.get(e)?),
                None => None,
            },
            ..self.clone()
        })
    }
}
