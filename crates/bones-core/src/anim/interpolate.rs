//! Interpolation entre poses : copie, mélange linéaire et Catmull-Rom.

use glam::{Quat, Vec4};

use crate::{BoneId, Pose, Transform};

pub(super) fn copy_pose(src: &Pose, out: &mut Pose) {
    out.bones.clear();
    out.bones.extend_from_slice(&src.bones);
}

pub(super) fn blend_into(a: &Pose, b: &Pose, t: f32, out: &mut Pose) {
    let len = a.bones.len().max(b.bones.len());
    out.bones.clear();
    out.bones
        .extend((0..len as u32).map(|i| a.get(BoneId(i)).lerp(&b.get(BoneId(i)), t)));
}

pub(super) fn catmull_rom(
    p0: &Transform,
    p1: &Transform,
    p2: &Transform,
    p3: &Transform,
    t: f32,
) -> Transform {
    let cr = |a: Vec4, b: Vec4, c: Vec4, d: Vec4| {
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * ((2.0 * b)
            + (c - a) * t
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
            + (3.0 * b - a - 3.0 * c + d) * t3)
    };
    // Les quaternions voisins sont ramenés dans le même hémisphère.
    let align = |q: Quat, reference: Quat| if q.dot(reference) < 0.0 { -q } else { q };
    let q1 = p1.rotation;
    let q0 = align(p0.rotation, q1);
    let q2 = align(p2.rotation, q1);
    let q3 = align(p3.rotation, q2);
    let rotation = cr(q0.into(), q1.into(), q2.into(), q3.into())
        .try_normalize()
        .map(Quat::from_vec4)
        .unwrap_or(q1);
    Transform {
        translation: cr(
            p0.translation.extend(0.0),
            p1.translation.extend(0.0),
            p2.translation.extend(0.0),
            p3.translation.extend(0.0),
        )
        .truncate(),
        rotation,
        scale: cr(
            p0.scale.extend(0.0),
            p1.scale.extend(0.0),
            p2.scale.extend(0.0),
            p3.scale.extend(0.0),
        )
        .truncate(),
    }
}
