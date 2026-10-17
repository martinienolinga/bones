use super::*;
use glam::Vec3;

use crate::{BoneId, Pose, Skeleton, head_tail};

fn arm() -> (Skeleton, Vec<BoneId>) {
    let mut s = Skeleton::new();
    let a = s.add_bone_world("a", None, Vec3::ZERO, Vec3::Y);
    let b = s.add_bone_world("b", Some(a), Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
    (s, vec![a, b])
}

fn tail(s: &Skeleton, pose: &Pose, id: BoneId) -> Vec3 {
    let w = pose.world(s);
    head_tail(&w[id.index()], s.bone(id).length).1
}

#[test]
fn reaches_target_and_keeps_lengths() {
    let (s, chain) = arm();
    let mut pose = Pose::rest(&s);
    let target = Vec3::new(1.0, 1.0, 0.3);
    assert!(solve_fabrik(&s, &mut pose, &chain, target, IkSettings::default()));
    assert!(tail(&s, &pose, chain[1]).abs_diff_eq(target, 1e-3));

    let w = pose.world(&s);
    let elbow = w[chain[1].index()].translation;
    assert!((elbow.length() - 1.0).abs() < 1e-4);
    assert!(w[chain[0].index()].translation.abs_diff_eq(Vec3::ZERO, 1e-6));
}

#[test]
fn unreachable_target_straightens_chain() {
    let (s, chain) = arm();
    let mut pose = Pose::rest(&s);
    assert!(!solve_fabrik(
        &s,
        &mut pose,
        &chain,
        Vec3::new(5.0, 0.0, 0.0),
        IkSettings::default()
    ));
    assert!(tail(&s, &pose, chain[1]).abs_diff_eq(Vec3::new(2.0, 0.0, 0.0), 1e-4));
}

#[test]
fn two_bone_reaches_target_and_respects_pole() {
    let (s, chain) = arm();
    let mut pose = Pose::rest(&s);
    let target = Vec3::new(0.0, 1.2, 0.5);
    let pole = Vec3::new(0.0, 0.6, -2.0);
    solve_two_bone(&s, &mut pose, chain[0], chain[1], target, pole);
    assert!(tail(&s, &pose, chain[1]).abs_diff_eq(target, 1e-4));
    let elbow = pose.world(&s)[chain[1].index()].translation;
    assert!((elbow.length() - 1.0).abs() < 1e-4);
    // Le coude part du côté du pôle (z négatif).
    assert!(elbow.z < 0.0, "{elbow:?}");
}

#[test]
fn pins_hold_their_target() {
    let (s, chain) = arm();
    let mut pose = Pose::rest(&s);
    let pin = Pin {
        bone: chain[1],
        target: Vec3::new(0.5, 1.5, 0.0),
    };
    solve_pins(&s, &mut pose, &[pin], 2, &[]);
    assert!(tail(&s, &pose, chain[1]).abs_diff_eq(pin.target, 1e-3));
}

#[test]
fn single_bone_points_at_target() {
    let (s, chain) = arm();
    let mut pose = Pose::rest(&s);
    solve_fabrik(
        &s,
        &mut pose,
        &chain[1..],
        Vec3::new(1.0, 1.0, 0.0),
        IkSettings::default(),
    );
    assert!(tail(&s, &pose, chain[1]).abs_diff_eq(Vec3::new(1.0, 1.0, 0.0), 1e-4));
}
