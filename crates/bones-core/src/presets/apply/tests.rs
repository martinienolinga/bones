use glam::Vec3;

use super::*;
use crate::presets::create::CreateOptions;
use crate::presets::model::PresetKind;
use crate::presets::{RefRig, builtin, find_builtin};
use crate::{Editor, Rig};

fn preset(id: &str) -> Preset {
    find_builtin(id).unwrap_or_else(|| panic!("{id}")).clone()
}

fn root_x(e: &Editor, frame: f32) -> Vec3 {
    let mut pose = e.rig.pose.clone();
    e.rig.clip.sample_into(frame, &mut pose);
    let root = e.rig.skeleton.find("root.x").unwrap();
    pose.world(&e.rig.skeleton)[root.index()].translation
}

#[test]
fn apply_is_a_single_undo_step() {
    let mut e = Editor::new(Rig::demo());
    let before = (*e.rig.clip).clone();
    let edits = e.edit_count();
    let report = e.apply_preset(&preset("builtin.human.walk"), &ApplyOptions::default()).unwrap();
    assert!(report.frames > 20);
    assert!(report.unmapped.len() < e.rig.skeleton.len());
    assert_eq!(e.rig.clip.keys().len() as u32, report.frames);
    assert!(e.edit_count() > edits);
    e.undo();
    assert_eq!(*e.rig.clip, before);
    assert!(!e.can_undo());
    e.redo();
    assert_eq!(e.rig.clip.keys().len() as u32, report.frames);
}

#[test]
fn failed_apply_changes_nothing() {
    let mut rig = Rig::new("Rope", crate::Mesh::default());
    rig.set_skeleton(RefRig::Rope.skeleton(), Vec::new());
    let mut e = Editor::new(rig);
    let err = e.apply_preset(&preset("builtin.human.walk"), &ApplyOptions::default());
    assert!(err.is_err());
    assert!(!e.can_undo());
}

#[test]
fn loops_accumulate_root_motion_and_in_place_removes_it() {
    let mut e = Editor::new(Rig::demo());
    let walk = preset("builtin.human.walk");
    let options = ApplyOptions {
        loops: 3,
        ..ApplyOptions::default()
    };
    let one = {
        let mut e1 = Editor::new(Rig::demo());
        e1.apply_preset(&walk, &ApplyOptions::default()).unwrap().frames
    };
    let report = e.apply_preset(&walk, &options).unwrap();
    assert_eq!(report.frames, 3 * (one - 1) + 1);
    let travel = root_x(&e, (report.frames - 1) as f32) - root_x(&e, 0.0);
    assert!(travel.z > 3.0, "{travel:?}");
    let options = ApplyOptions {
        loops: 2,
        in_place: true,
        ..ApplyOptions::default()
    };
    let report = e.apply_preset(&walk, &options).unwrap();
    let travel = root_x(&e, (report.frames - 1) as f32) - root_x(&e, 0.0);
    assert!(Vec3::new(travel.x, 0.0, travel.z).length() < 1e-3, "{travel:?}");
}

#[test]
fn insert_shifts_later_keys_and_continues_from_the_character() {
    let mut e = Editor::new(Rig::demo());
    let walk = preset("builtin.human.walk");
    let n = e.apply_preset(&walk, &ApplyOptions::default()).unwrap().frames;
    let end = root_x(&e, (n - 1) as f32);
    let options = ApplyOptions {
        mode: ApplyMode::Insert,
        frame: n,
        ..ApplyOptions::default()
    };
    let report = e.apply_preset(&preset("builtin.human.jump"), &options).unwrap();
    assert_eq!(report.start, n);
    // Le saut part de là où la marche s'arrête.
    let start = root_x(&e, n as f32);
    assert!((Vec3::new(start.x - end.x, 0.0, start.z - end.z)).length() < 0.05, "{start:?} / {end:?}");
    assert_eq!(e.rig.clip.frame_count, n + report.frames);
}

#[test]
fn blend_and_bone_mask_keep_the_rest_of_the_animation() {
    let mut e = Editor::new(Rig::demo());
    let walk = preset("builtin.human.walk");
    let n = e.apply_preset(&walk, &ApplyOptions::default()).unwrap().frames;
    let walked = (*e.rig.clip).clone();
    let arm: Vec<_> = ["shoulder.r", "arm.r", "forearm.r", "hand.r"]
        .iter()
        .map(|b| e.rig.skeleton.find(b).unwrap())
        .collect();
    let options = ApplyOptions {
        mode: ApplyMode::Blend,
        frame: 0,
        bones: arm.clone(),
        ..ApplyOptions::default()
    };
    e.apply_preset(&preset("builtin.human.wave"), &options).unwrap();
    let (mut a, mut b) = (e.rig.pose.clone(), e.rig.pose.clone());
    walked.sample_into(10.0, &mut a);
    e.rig.clip.sample_into(10.0, &mut b);
    let thigh = e.rig.skeleton.find("thigh.l").unwrap();
    assert!(a.bones[thigh.index()].abs_diff_eq(&b.bones[thigh.index()], 1e-5));
    assert!(!a.bones[arm[1].index()].abs_diff_eq(&b.bones[arm[1].index()], 1e-3));
    assert!(e.rig.clip.frame_count >= n);
}

#[test]
fn mirror_raises_the_other_hand() {
    let skeleton = RefRig::Humanoid.skeleton();
    let roles = crate::presets::detect(&skeleton);
    let base = crate::Pose::rest(&skeleton);
    let wave = preset("builtin.human.wave");
    let hand = |mirror| {
        let options = ApplyOptions {
            mirror,
            ..ApplyOptions::default()
        };
        let (poses, _) = bake_preset(&wave, &skeleton, &roles, &base, &options, 30.0).unwrap();
        let w = poses[0].world(&skeleton);
        let (l, r) = (skeleton.find("hand.l").unwrap(), skeleton.find("hand.r").unwrap());
        (w[l.index()].translation.y, w[r.index()].translation.y)
    };
    let (l, r) = hand(false);
    assert!(r > l + 0.3);
    let (l, r) = hand(true);
    assert!(l > r + 0.3);
}

#[test]
fn created_preset_reapplies_identically() {
    let mut e = Editor::new(Rig::demo());
    e.apply_preset(&preset("builtin.human.dance"), &ApplyOptions::default()).unwrap();
    let created = e
        .create_preset(&CreateOptions {
            name: "My/Dance:1".into(),
            looping: true,
            ..CreateOptions::default()
        })
        .unwrap();
    assert_eq!(created.info.name, "My-Dance-1");
    assert!(matches!(created.kind, PresetKind::Keyframes(_)));
    let dense = (*e.rig.clip).clone();
    e.apply_preset(&created, &ApplyOptions::default()).unwrap();
    let (mut a, mut b) = (e.rig.pose.clone(), e.rig.pose.clone());
    for f in [0.0, 7.0, 20.0] {
        dense.sample_into(f, &mut a);
        e.rig.clip.sample_into(f, &mut b);
        let (wa, wb) = (a.world(&e.rig.skeleton), b.world(&e.rig.skeleton));
        for (x, y) in wa.iter().zip(&wb) {
            assert!(x.translation.abs_diff_eq(y.translation, 2e-3));
        }
    }
}

#[test]
fn builtin_library_is_listed_cheaply() {
    let all = builtin();
    assert!(all.len() >= 40);
    for section in crate::presets::Section::ALL {
        let count = all.iter().filter(|p| section.contains(&p.info)).count();
        if matches!(section, crate::presets::Section::Category(_)) {
            assert!(count >= 5, "{section:?}");
        }
    }
    let ids: std::collections::HashSet<_> = all.iter().map(|p| &p.info.id).collect();
    assert_eq!(ids.len(), all.len());
    assert!(all.iter().all(|p| p.info.license == "CC0-1.0"));
}

#[test]
fn previews_draw_every_builtin_preset() {
    let mut segments = Vec::new();
    for preset in builtin().iter().take(48) {
        let preview = crate::presets::Preview::new(preset).unwrap();
        preview.segments(preview.duration() * 0.5, &mut segments);
        assert_eq!(segments.len(), preview.skeleton().len());
        assert!(segments.iter().all(|[a, b]| a.is_finite() && b.is_finite()));
    }
}
