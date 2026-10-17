//! Pose : deltas par os appliqués après la pose de repos, et cinématique directe.

use glam::{Mat4, Quat};
use serde::{Deserialize, Serialize};

use crate::{BoneId, BoneRemap, Skeleton, Transform};

/// Pose d'un squelette, exprimée comme un delta par os appliqué *après* la
/// transformation de repos (comme le mode pose de Blender).
///
/// L'identité partout correspond donc à la pose de repos, et une pose reste
/// valable si l'on retouche le squelette en mode édition.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub bones: Vec<Transform>,
}

impl Pose {
    pub fn rest(skeleton: &Skeleton) -> Self {
        Self {
            bones: vec![Transform::IDENTITY; skeleton.len()],
        }
    }

    pub fn get(&self, id: BoneId) -> Transform {
        self.bones.get(id.index()).copied().unwrap_or_default()
    }

    pub fn set(&mut self, id: BoneId, t: Transform) {
        self.bones[id.index()] = t;
    }

    /// Ajuste la taille de la pose à celle du squelette.
    pub fn sync_len(&mut self, skeleton: &Skeleton) {
        self.bones.resize(skeleton.len(), Transform::IDENTITY);
    }

    /// Cinématique directe : transformations monde de chaque os.
    pub fn world(&self, skeleton: &Skeleton) -> Vec<Transform> {
        let mut world: Vec<Transform> = Vec::with_capacity(skeleton.len());
        for (i, bone) in skeleton.bones().iter().enumerate() {
            let local = bone.rest.mul(&self.get(BoneId(i as u32)));
            let w = match bone.parent() {
                Some(p) => world[p.index()].mul(&local),
                None => local,
            };
            world.push(w);
        }
        world
    }

    /// Matrices de skinning : passent de l'espace monde de repos à l'espace
    /// monde posé.
    pub fn skinning_matrices(&self, skeleton: &Skeleton) -> Vec<Mat4> {
        let rest = skeleton.rest_world();
        self.world(skeleton)
            .iter()
            .zip(&rest)
            .map(|(posed, rest)| posed.to_matrix() * rest.to_matrix().inverse())
            .collect()
    }

    /// Applique une rotation exprimée en espace monde à un os, autour de sa tête.
    ///
    /// `parent_world` est la transformation monde courante du parent (ou
    /// l'identité pour une racine).
    pub fn rotate_world(
        &mut self,
        skeleton: &Skeleton,
        id: BoneId,
        parent_world: &Transform,
        rotation: Quat,
    ) {
        let basis = (parent_world.rotation * skeleton.bone(id).rest.rotation).normalize();
        let delta = &mut self.bones[id.index()];
        delta.rotation = (basis.inverse() * rotation * basis * delta.rotation).normalize();
    }

    pub fn remap(&mut self, remap: &BoneRemap) {
        let mut bones = Vec::with_capacity(self.bones.len().saturating_sub(1));
        for (old, t) in self.bones.iter().enumerate() {
            if remap.map.get(old).copied().flatten().is_some() {
                bones.push(*t);
            }
        }
        self.bones = bones;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::head_tail;
    use glam::Vec3;
    use std::f32::consts::FRAC_PI_2;

    #[test]
    fn rest_pose_has_identity_skinning() {
        let mut s = Skeleton::new();
        let a = s.add_bone_world("a", None, Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0));
        s.add_bone_world("b", Some(a), Vec3::new(1.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 0.0));
        let pose = Pose::rest(&s);
        for m in pose.skinning_matrices(&s) {
            assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-5));
        }
    }

    #[test]
    fn rotate_world_turns_child() {
        let mut s = Skeleton::new();
        let a = s.add_bone_world("a", None, Vec3::ZERO, Vec3::Y);
        let b = s.add_bone_world("b", Some(a), Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
        let mut pose = Pose::rest(&s);
        // Rotation de 90° autour de Z : l'os pointe vers -X.
        pose.rotate_world(&s, a, &Transform::IDENTITY, Quat::from_rotation_z(FRAC_PI_2));
        let w = pose.world(&s);
        let (_, tail_b) = head_tail(&w[b.index()], s.bone(b).length);
        assert!(tail_b.abs_diff_eq(Vec3::new(-2.0, 0.0, 0.0), 1e-5));

        // La rotation monde d'un enfant tient compte de la pose du parent.
        let parent = w[a.index()];
        pose.rotate_world(&s, b, &parent, Quat::from_rotation_z(-FRAC_PI_2));
        let w = pose.world(&s);
        let (_, tail_b) = head_tail(&w[b.index()], s.bone(b).length);
        assert!(tail_b.abs_diff_eq(Vec3::new(-1.0, 1.0, 0.0), 1e-5));
    }
}
