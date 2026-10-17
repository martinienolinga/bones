//! Déplacement du corps : position monde du bassin, décalage horizontal
//! (alignement sur la position actuelle), suppression du déplacement
//! (animation sur place).

use glam::Vec3;

use crate::presets::roles::RoleMap;
use crate::{BoneId, Pose, Skeleton};

/// Os qui porte le déplacement : le bassin, sinon la première racine.
pub(super) fn root_bone(skeleton: &Skeleton, roles: &RoleMap) -> BoneId {
    roles
        .root()
        .filter(|r| roles.fits(skeleton) && skeleton.contains(*r))
        .unwrap_or(BoneId(0))
}

/// Position monde de l'os (tête) dans une pose.
pub(super) fn position(skeleton: &Skeleton, pose: &Pose, bone: BoneId) -> Vec3 {
    // Seule la branche de l'os compte : composition de la racine à l'os.
    let mut chain = vec![bone];
    let mut cur = skeleton.bone(bone).parent();
    while let Some(p) = cur {
        chain.push(p);
        cur = skeleton.bone(p).parent();
    }
    let mut world = crate::Transform::IDENTITY;
    for b in chain.into_iter().rev() {
        world = world.mul(&skeleton.bone(b).rest.mul(&pose.get(b)));
    }
    world.translation
}

/// Décale l'os d'un vecteur monde (son parent étant au repos).
pub(super) fn shift(skeleton: &Skeleton, pose: &mut Pose, bone: BoneId, offset: Vec3) {
    let mut rest = crate::Transform::IDENTITY;
    let mut cur = Some(bone);
    let mut chain = Vec::new();
    while let Some(b) = cur {
        chain.push(b);
        cur = skeleton.bone(b).parent();
    }
    for b in chain.into_iter().rev() {
        rest = rest.mul(&skeleton.bone(b).rest);
    }
    pose.bones[bone.index()].translation += rest.rotation.inverse() * offset / rest.scale;
}

/// Composante horizontale d'un vecteur.
pub(super) fn flat(roles: &RoleMap, v: Vec3) -> Vec3 {
    let up = roles.frame.up;
    v - up * v.dot(up)
}

/// Retire la dérive horizontale du bassin entre la première et la dernière
/// image (marche sur place) ; les oscillations sont conservées.
pub(super) fn strip_travel(skeleton: &Skeleton, roles: &RoleMap, poses: &mut [Pose]) {
    if poses.len() < 2 {
        return;
    }
    let root = root_bone(skeleton, roles);
    let first = position(skeleton, &poses[0], root);
    let last = position(skeleton, &poses[poses.len() - 1], root);
    let drift = flat(roles, last - first);
    let n = (poses.len() - 1) as f32;
    for (i, pose) in poses.iter_mut().enumerate() {
        shift(skeleton, pose, root, -drift * (i as f32 / n));
    }
}

/// Déplace toute l'animation horizontalement pour que la première image
/// parte de `at` (position monde du bassin).
pub(super) fn start_at(skeleton: &Skeleton, roles: &RoleMap, poses: &mut [Pose], at: Vec3) {
    let Some(first) = poses.first() else { return };
    let root = root_bone(skeleton, roles);
    let offset = flat(roles, at - position(skeleton, first, root));
    for pose in poses.iter_mut() {
        shift(skeleton, pose, root, offset);
    }
}
