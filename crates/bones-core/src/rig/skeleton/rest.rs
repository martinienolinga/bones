//! Pose de repos : transformations monde et connexion des os.

use glam::{Quat, Vec3};

use super::{BoneId, Skeleton};
use crate::Transform;

impl Skeleton {
    /// Vrai si l'enfant `child` est collé à la queue de son parent.
    pub fn is_connected(&self, child: BoneId) -> bool {
        let bone = self.bone(child);
        let Some(parent) = bone.parent.filter(|_| !bone.detached) else {
            return false;
        };
        let len = self.bone(parent).length;
        bone.rest
            .translation
            .abs_diff_eq(Vec3::Y * len, 1e-4 * len.max(1.0))
    }

    /// Transformations monde de la pose de repos.
    pub fn rest_world(&self) -> Vec<Transform> {
        let mut world: Vec<Transform> = Vec::with_capacity(self.bones.len());
        for bone in &self.bones {
            let w = match bone.parent {
                Some(p) => world[p.index()].mul(&bone.rest),
                None => bone.rest,
            };
            world.push(w);
        }
        world
    }

    pub(super) fn rest_world_of(&self, id: BoneId) -> Transform {
        let bone = self.bone(id);
        match bone.parent {
            Some(p) => self.rest_world_of(p).mul(&bone.rest),
            None => bone.rest,
        }
    }

    /// Rotation monde de repos d'un os (utile pour les outils d'édition).
    pub fn rest_world_rotation(&self, id: BoneId) -> Quat {
        self.rest_world_of(id).rotation
    }
}
