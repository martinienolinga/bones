use super::*;
use glam::Vec3;

use crate::{Pose, Transform};

fn pose_t(x: f32) -> Pose {
    Pose {
        bones: vec![Transform::from_translation(Vec3::new(x, 0.0, 0.0))],
    }
}

#[test]
fn keys_stay_sorted_and_unique() {
    let mut c = Clip::default();
    c.set_key(10, pose_t(1.0));
    c.set_key(0, pose_t(0.0));
    c.set_key(10, pose_t(2.0));
    assert_eq!(c.keys().iter().map(|k| k.frame).collect::<Vec<_>>(), vec![0, 10]);
    assert_eq!(c.key_at(10).unwrap().pose, pose_t(2.0));
    assert_eq!(c.interval_at(4), Some((0, 10)));
    assert_eq!(c.interval_at(10), None);
    assert_eq!(c.next_key_frame(0), Some(10));
    assert_eq!(c.prev_key_frame(10), Some(0));
}

#[test]
fn interpolation_modes() {
    let mut c = Clip::default();
    c.set_key(0, pose_t(0.0));
    c.set_key(10, pose_t(10.0));
    let mut out = Pose::default();

    c.key_at_mut(0).unwrap().interpolation = Interpolation::Linear;
    c.sample_into(5.0, &mut out);
    assert!((out.bones[0].translation.x - 5.0).abs() < 1e-5);

    c.key_at_mut(0).unwrap().interpolation = Interpolation::Step;
    c.sample_into(9.0, &mut out);
    assert_eq!(out.bones[0].translation.x, 0.0);

    c.key_at_mut(0).unwrap().interpolation = Interpolation::Smooth;
    c.sample_into(0.0, &mut out);
    assert_eq!(out.bones[0].translation.x, 0.0);
    c.sample_into(10.0, &mut out);
    assert_eq!(out.bones[0].translation.x, 10.0);
    c.sample_into(5.0, &mut out);
    assert!((out.bones[0].translation.x - 5.0).abs() < 1e-4);
}
