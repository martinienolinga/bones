use glam::{Quat, Vec3};

use super::*;
use crate::presets::model::KeyframeData;
use crate::presets::params::Params;
use crate::presets::procedural::{Generator, bake};
use crate::presets::reference::RefRig;
use crate::presets::roles::detect;
use crate::{Pose, Skeleton, Transform, head_tail};

/// Variante en pose A : chaque bras tourné de 45° vers le bas autour de l'épaule.
fn a_pose(mut s: Skeleton) -> Skeleton {
    let mut world = s.rest_world();
    for (name, sign) in [("arm.l", -1.0f32), ("arm.r", 1.0)] {
        let arm = s.find(name).unwrap();
        let pivot = world[arm.index()].translation;
        let q = Quat::from_rotation_z(sign * 45f32.to_radians());
        for b in s.subtree(arm) {
            let w = &mut world[b.index()];
            w.translation = pivot + q * (w.translation - pivot);
            w.rotation = (q * w.rotation).normalize();
        }
    }
    s.set_rest_world(&world);
    s
}

fn walk_data(skeleton: Skeleton) -> KeyframeData {
    let roles = detect(&skeleton);
    let clip = bake(Generator::HumanLocomotion, &Params::of(&[("speed", 1.4)]), &skeleton, &roles, 30.0, 1.0, 1).unwrap();
    KeyframeData {
        skeleton,
        roles,
        clip,
        bones: Vec::new(),
    }
}

fn retarget(data: &KeyframeData, target: &Skeleton, options: RetargetOptions) -> (Vec<Pose>, Report) {
    let roles = detect(target);
    retarget_clip(data, true, (target, &roles), &options, &Pose::rest(target), 30.0, 1.0, 1).unwrap()
}

/// Directions monde des os nommés `names` dans une pose.
fn dirs(s: &Skeleton, pose: &Pose, names: &[&str]) -> Vec<Vec3> {
    let w = pose.world(s);
    names
        .iter()
        .map(|n| {
            let b = s.find(n).unwrap();
            let (h, t) = head_tail(&w[b.index()], s.bone(b).length);
            (t - h).normalize()
        })
        .collect()
}

const BONES: &[&str] = &["arm.l", "forearm.r", "thigh.l", "leg.r", "spine_02.x", "head.x"];

#[test]
fn identity_on_the_same_skeleton() {
    let data = walk_data(RefRig::Humanoid.skeleton());
    let (poses, report) = retarget(&data, &data.skeleton, RetargetOptions::default());
    assert!(report.unmapped.is_empty(), "{:?}", report.unmapped);
    let mut src = Pose::rest(&data.skeleton);
    for (f, pose) in poses.iter().enumerate() {
        data.clip.sample_into(f as f32, &mut src);
        let (a, b) = (src.world(&data.skeleton), pose.world(&data.skeleton));
        for (x, y) in a.iter().zip(&b) {
            assert!(x.translation.abs_diff_eq(y.translation, 2e-3), "image {f}");
        }
    }
}

#[test]
fn a_pose_source_drives_t_pose_target_and_back() {
    let t_pose = RefRig::Humanoid.skeleton();
    let data = walk_data(a_pose(t_pose.clone()));
    let (poses, _) = retarget(&data, &t_pose, RetargetOptions::default());
    let mut src = Pose::rest(&data.skeleton);
    for (f, pose) in poses.iter().enumerate() {
        data.clip.sample_into(f as f32, &mut src);
        for (a, b) in dirs(&data.skeleton, &src, BONES).iter().zip(dirs(&t_pose, pose, BONES)) {
            assert!(a.angle_between(b).to_degrees() < 1.0, "image {f}: {a:?} / {b:?}");
        }
    }
    // Retour : T → A redonne l'animation d'origine.
    let roles = detect(&t_pose);
    let mut clip = crate::Clip::new(30.0, poses.len() as u32);
    for (f, p) in poses.into_iter().enumerate() {
        clip.set_key(f as u32, p);
    }
    let back = KeyframeData {
        skeleton: t_pose,
        roles,
        clip,
        bones: Vec::new(),
    };
    let (again, _) = retarget(&back, &data.skeleton, RetargetOptions::default());
    for (f, pose) in again.iter().enumerate() {
        data.clip.sample_into(f as f32, &mut src);
        let (a, b) = (src.world(&data.skeleton), pose.world(&data.skeleton));
        for (x, y) in a.iter().zip(&b) {
            assert!(x.translation.abs_diff_eq(y.translation, 3e-3), "image {f}");
        }
    }
}

#[test]
fn bigger_target_scales_root_motion_and_keeps_feet_planted() {
    let data = walk_data(RefRig::Humanoid.skeleton());
    let mut big = data.skeleton.clone();
    let world: Vec<Transform> = big
        .rest_world()
        .iter()
        .map(|w| Transform::from_rotation_translation(w.rotation, w.translation * 1.5))
        .collect();
    for b in big.ids().collect::<Vec<_>>() {
        big.bone_mut(b).length *= 1.5;
    }
    big.set_rest_world(&world);
    let (poses, _) = retarget(&data, &big, RetargetOptions::default());
    let root = big.find("root.x").unwrap();
    let travel = |s: &Skeleton, a: &Pose, b: &Pose| {
        (b.world(s)[root.index()].translation - a.world(s)[root.index()].translation).z
    };
    let (mut s0, mut s1) = (Pose::rest(&data.skeleton), Pose::rest(&data.skeleton));
    data.clip.sample_into(0.0, &mut s0);
    data.clip.sample_into((poses.len() - 1) as f32, &mut s1);
    let ratio = travel(&big, &poses[0], &poses[poses.len() - 1]) / travel(&data.skeleton, &s0, &s1);
    assert!((ratio - 1.5).abs() < 0.02, "{ratio}");
    let roles = detect(&big);
    let mut clip = crate::Clip::new(30.0, poses.len() as u32);
    for (f, p) in poses.into_iter().enumerate() {
        clip.set_key(f as u32, p);
    }
    let contacts = crate::presets::procedural::tests::check_planted(&big, &roles, &clip, "grand");
    assert!(contacts > 5);
}

#[test]
fn mixamo_names_map_completely() {
    let data = walk_data(RefRig::Humanoid.skeleton());
    let mut target = RefRig::Humanoid.skeleton();
    for b in target.ids().collect::<Vec<_>>() {
        let name = target.bone(b).name.clone();
        let mixamo = match name.as_str() {
            "root.x" => "mixamorig:Hips".to_owned(),
            n => format!("mixamorig:{}", n.replace(".l", "Left").replace(".r", "Right")),
        };
        target.bone_mut(b).name = mixamo;
    }
    let (_, report) = retarget(&data, &target, RetargetOptions::default());
    let arms = report.mapped.iter().filter(|(t, _)| t.contains("arm")).count();
    assert!(arms >= 4, "{:?}", report.mapped);
}

#[test]
fn mirror_swaps_sides() {
    let data = walk_data(RefRig::Humanoid.skeleton());
    let options = RetargetOptions {
        mirror: true,
        ..RetargetOptions::default()
    };
    let (poses, _) = retarget(&data, &data.skeleton, options);
    let s = &data.skeleton;
    let mut src = Pose::rest(s);
    data.clip.sample_into(7.0, &mut src);
    let (a, b) = (src.world(s), poses[7].world(s));
    let (l, r) = (s.find("foot.l").unwrap(), s.find("foot.r").unwrap());
    let flip = |v: Vec3| Vec3::new(-v.x, v.y, v.z);
    assert!(flip(a[l.index()].translation).abs_diff_eq(b[r.index()].translation, 3e-3));
}
