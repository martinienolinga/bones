use super::*;
use glam::Vec3;

use crate::BoneId;
use crate::Pose;
use glam::Quat;

fn tube_rig() -> (Mesh, Skeleton) {
    let mesh = Mesh::capsule(Vec3::ZERO, Vec3::new(0.0, 2.0, 0.0), 0.2, 12, 8);
    let mut s = Skeleton::new();
    let a = s.add_bone_world("a", None, Vec3::ZERO, Vec3::Y);
    s.add_bone_world("b", Some(a), Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
    (mesh, s)
}

#[test]
fn quantized_weights_sum_to_one() {
    let vw = VertexWeights::from_influences(vec![
        (BoneId(3), 0.3),
        (BoneId(1), 0.3),
        (BoneId(2), 0.3),
        (BoneId(0), 0.05),
        (BoneId(4), 0.05),
    ]);
    let sum: u32 = vw.weights.iter().map(|w| *w as u32).sum();
    assert_eq!(sum, u16::MAX as u32);
    assert_eq!(std::mem::size_of::<VertexWeights>(), 16);
}

#[test]
fn automatic_weights_are_normalized_and_local() {
    let (mesh, s) = tube_rig();
    let skin = Skin::automatic(&mesh, &s);
    assert_eq!(skin.weights.len(), mesh.positions.len());
    for (p, vw) in mesh.positions.iter().zip(&skin.weights) {
        let total: f32 = vw.iter().map(|(_, w)| w).sum();
        assert!((total - 1.0).abs() < 1e-4);
        if p.y < 0.5 {
            assert!(vw.weight_of(BoneId(0)) > 0.9);
        }
        if p.y > 1.5 {
            assert!(vw.weight_of(BoneId(1)) > 0.9);
        }
    }
}

#[test]
fn rest_pose_does_not_deform() {
    let (mesh, s) = tube_rig();
    let skin = Skin::automatic(&mesh, &s);
    let (mut p, mut n) = (Vec::new(), Vec::new());
    skin.deform_into(&mesh, &Pose::rest(&s).skinning_matrices(&s), &mut p, &mut n);
    for (a, b) in p.iter().zip(&mesh.positions) {
        assert!(a.abs_diff_eq(*b, 1e-5));
    }
}

#[test]
fn posed_bone_moves_its_vertices() {
    let (mesh, s) = tube_rig();
    let skin = Skin::automatic(&mesh, &s);
    let mut pose = Pose::rest(&s);
    let parent = pose.world(&s)[0];
    pose.rotate_world(
        &s,
        BoneId(1),
        &parent,
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
    );
    let (mut p, mut n) = (Vec::new(), Vec::new());
    skin.deform_into(&mesh, &pose.skinning_matrices(&s), &mut p, &mut n);
    // Le pôle du haut (y = 2.2) part vers -X, le pôle du bas ne bouge pas.
    let top = mesh.positions.iter().position(|v| v.y > 2.19).unwrap();
    let bottom = mesh.positions.iter().position(|v| v.y < -0.19).unwrap();
    assert!(p[top].x < -1.0);
    assert!(p[bottom].abs_diff_eq(mesh.positions[bottom], 1e-4));
}

#[test]
fn remap_moves_influence_to_parent() {
    let (mesh, mut s) = tube_rig();
    let mut skin = Skin::automatic(&mesh, &s);
    let remap = s.remove_bone(BoneId(1));
    skin.remap(&remap);
    for vw in &skin.weights {
        assert!((vw.weight_of(BoneId(0)) - 1.0).abs() < 1e-4);
    }
}

#[test]
fn bone_masses_follow_weights() {
    let (mesh, s) = tube_rig();
    let skin = Skin::automatic(&mesh, &s);
    let masses = skin.bone_masses(s.len());
    let total: f32 = masses.iter().sum();
    assert!((total - mesh.positions.len() as f32).abs() < 0.01 * total);
}
