//! Sélection au doigt : on travaille en espace écran, plus tolérant qu'un
//! lancer de rayon pour des cibles fines comme les os.

use glam::Vec2;

use crate::{BoneId, OrbitCamera, Skeleton, Transform, Viewport, head_tail};

pub fn distance_to_segment_2d(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t - p).length()
}

/// Os le plus proche du point écran `screen`, à moins de `radius` pixels.
pub fn pick_bone(
    skeleton: &Skeleton,
    world: &[Transform],
    camera: &OrbitCamera,
    viewport: &Viewport,
    screen: Vec2,
    radius: f32,
) -> Option<BoneId> {
    skeleton
        .ids()
        .filter_map(|id| {
            let (head, tail) = head_tail(&world[id.index()], skeleton.bone(id).length);
            let a = camera.project(head, viewport)?;
            let b = camera.project(tail, viewport)?;
            let d = distance_to_segment_2d(screen, a, b);
            (d <= radius).then_some((id, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn picks_nearest_bone() {
        let mut s = Skeleton::new();
        let a = s.add_bone_world("a", None, Vec3::ZERO, Vec3::Y);
        let b = s.add_bone_world("b", Some(a), Vec3::Y, Vec3::new(1.0, 1.0, 0.0));
        let world = s.rest_world();
        let cam = OrbitCamera {
            yaw: 0.0,
            pitch: 0.0,
            ..Default::default()
        };
        let vp = Viewport::new(800.0, 600.0);
        let mid_b = cam.project(Vec3::new(0.5, 1.0, 0.0), &vp).unwrap();
        assert_eq!(pick_bone(&s, &world, &cam, &vp, mid_b, 20.0), Some(b));
        let mid_a = cam.project(Vec3::new(0.0, 0.5, 0.0), &vp).unwrap();
        assert_eq!(pick_bone(&s, &world, &cam, &vp, mid_a, 20.0), Some(a));
        assert_eq!(pick_bone(&s, &world, &cam, &vp, Vec2::new(5.0, 5.0), 20.0), None);
    }
}
