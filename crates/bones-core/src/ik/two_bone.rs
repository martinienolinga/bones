//! IK analytique à deux os (loi des cosinus) avec vecteur de pôle.

use glam::Vec3;

use crate::math::rotation_arc;
use crate::{BoneId, Pose, Skeleton, Transform, head_tail};

/// IK analytique à deux os (bras, jambe) avec vecteur de pôle, comme les
/// membres d'Auto-Rig Pro : le coude/genou est orienté vers `pole`.
///
/// `upper` doit être le parent direct de `lower`. La queue de `lower`
/// (poignet, cheville) est amenée sur `target`, ou au plus près.
pub fn solve_two_bone(
    skeleton: &Skeleton,
    pose: &mut Pose,
    upper: BoneId,
    lower: BoneId,
    target: Vec3,
    pole: Vec3,
) {
    debug_assert_eq!(skeleton.bone(lower).parent(), Some(upper));
    let world = pose.world(skeleton);
    let a = world[upper.index()].translation;
    let b = world[lower.index()].translation;
    let c = head_tail(&world[lower.index()], skeleton.bone(lower).length).1;
    let la = (b - a).length();
    let lb = (c - b).length();
    if la < 1e-6 || lb < 1e-6 {
        return;
    }

    let to_target = target - a;
    let dist = to_target.length().clamp((la - lb).abs() + 1e-5, la + lb - 1e-5);
    let dir = to_target.try_normalize().unwrap_or((c - a).normalize_or(Vec3::Y));
    // Plan de flexion : contient l'axe épaule→cible et le pôle.
    let bend = (pole - a - dir * (pole - a).dot(dir))
        .try_normalize()
        .or_else(|| (b - a - dir * (b - a).dot(dir)).try_normalize())
        .unwrap_or_else(|| dir.any_orthonormal_vector());
    // Loi des cosinus : angle à l'épaule.
    let cos_a = ((la * la + dist * dist - lb * lb) / (2.0 * la * dist)).clamp(-1.0, 1.0);
    let sin_a = (1.0 - cos_a * cos_a).sqrt();
    let elbow = a + (dir * cos_a + bend * sin_a) * la;
    let wrist = a + dir * dist;

    let parent_world = skeleton
        .bone(upper)
        .parent()
        .map(|p| world[p.index()])
        .unwrap_or(Transform::IDENTITY);
    pose.rotate_world(skeleton, upper, &parent_world, rotation_arc(b - a, elbow - a));

    let world = pose.world(skeleton);
    let b = world[lower.index()].translation;
    let c = head_tail(&world[lower.index()], skeleton.bone(lower).length).1;
    pose.rotate_world(
        skeleton,
        lower,
        &world[upper.index()],
        rotation_arc(c - b, wrist - b),
    );
}
