use glam::Vec3;

use super::*;
use crate::presets::{builtin, find_builtin};
use crate::samples::mannequin;
use crate::{Transform, Viewport};

fn model() -> (Rig, StageModel) {
    let rig = mannequin();
    let model = StageModel::new(&rig);
    (rig, model)
}

fn preset(id: &str) -> &'static Preset {
    find_builtin(id).unwrap_or_else(|| panic!("préréglage {id} absent"))
}

/// Position monde de la tête d'un os à l'image `frame`.
fn bone_position(staged: &Staged, model: &StageModel, frame: usize, name: &str) -> Vec3 {
    let id = model.skeleton().find(name).expect("os du mannequin");
    let world = staged.poses[frame].world(model.skeleton());
    world[id.index()].translation
}

#[test]
fn shots_follow_the_kind_of_preset() {
    assert_eq!(shot_for(preset("builtin.human.walk")), Some(Shot::Body));
    assert_eq!(shot_for(preset("builtin.face.happy")), Some(Shot::Face));
    assert_eq!(shot_for(preset("builtin.animals.dog-walk")), None);
    assert_eq!(shot_for(preset("builtin.vegetation.tree-breeze")), None);
}

#[test]
fn expressions_pose_only_the_face() {
    let (rig, model) = model();
    let face: Vec<_> = crate::presets::expression::present_mapped(&rig.skeleton)
        .into_iter()
        .map(|(_, id)| id.index())
        .collect();
    assert!(face.len() > 20, "le mannequin a un visage");
    let staged = Staged::new(preset("builtin.face.surprise"), &model).unwrap();
    assert!(staged.is_static());
    assert_eq!(staged.shot(), Shot::Face);
    let pose = &staged.poses[0];
    let moved: Vec<usize> = (0..pose.bones.len())
        .filter(|i| !pose.bones[*i].abs_diff_eq(&Transform::IDENTITY, 1e-6))
        .collect();
    assert!(!moved.is_empty());
    assert!(moved.iter().all(|i| face.contains(i)), "seul le visage bouge");

    let (mut world, mut joints) = (Vec::new(), Vec::new());
    staged.joints(&model, 0, &mut world, &mut joints);
    assert_eq!(joints.len(), rig.skeleton.len());
    assert!(joints.iter().all(|m| m.is_finite()));
    // Gros plan : de l'ordre d'une tête (quelques dizaines de centimètres).
    let half = staged.extent().half;
    assert!(half.x > 0.05 && half.x < 0.25, "largeur {half}");
    assert!(half.y > 0.08 && half.y < 0.3, "hauteur {half}");
    assert!(staged.extent().center.y > 1.3, "centré sur la tête");
}

#[test]
fn locomotion_is_played_in_place() {
    let (_, model) = model();
    let staged = Staged::new(preset("builtin.human.walk"), &model).unwrap();
    assert_eq!(staged.shot(), Shot::Body);
    assert!(staged.frames() > 10);
    let first = bone_position(&staged, &model, 0, "root.x");
    let last = bone_position(&staged, &model, staged.frames() - 1, "root.x");
    let drift = Vec3::new(last.x - first.x, 0.0, last.z - first.z).length();
    assert!(drift < 0.02, "dérive horizontale {drift}");
    // Corps entier : environ 1,8 m de haut.
    let half = staged.extent().half;
    assert!(half.y > 0.7 && half.y < 1.3, "demi-hauteur {half}");

    let (mut world, mut joints) = (Vec::new(), Vec::new());
    for frame in 0..staged.frames() {
        staged.joints(&model, frame, &mut world, &mut joints);
        assert!(joints.iter().all(|m| m.is_finite()));
    }
}

#[test]
fn all_humanoid_presets_and_expressions_stage() {
    let (_, model) = model();
    for p in builtin() {
        match shot_for(p) {
            Some(_) => {
                let staged = Staged::new(p, &model).unwrap_or_else(|e| panic!("{}: {e}", p.info.id));
                assert!(staged.extent().half.is_finite(), "{}", p.info.id);
            }
            None => assert!(Staged::new(p, &model).is_err(), "{}", p.info.id),
        }
    }
}

#[test]
fn frames_loop_or_hold_on_the_last_pose() {
    let (_, model) = model();
    let walk = Staged::new(preset("builtin.human.walk"), &model).unwrap();
    let n = walk.frames();
    assert_eq!(walk.frame_at(0.0), 0);
    assert_eq!(walk.frame_at((n - 1) as f32 / FPS), 0, "boucle");
    assert!(walk.frame_at(1e6) < n);
    assert_eq!(walk.frame_at(f32::NAN), 0);

    let jump = Staged::new(preset("builtin.human.jump"), &model).unwrap();
    let last = jump.frames() - 1;
    let end = last as f32 / FPS;
    assert_eq!(jump.frame_at(end + HOLD * 0.5), last, "pause sur la fin");
    assert_eq!(jump.frame_at(end + HOLD + 0.01), 0, "rejoue");
}

#[test]
fn camera_frames_the_whole_extent() {
    let (_, model) = model();
    for id in ["builtin.human.run", "builtin.face.happy"] {
        let staged = Staged::new(preset(id), &model).unwrap();
        for aspect in [0.75, 1.0, 1.6] {
            let camera = staged.camera(aspect, 0.1);
            let viewport = Viewport::new(100.0 * aspect, 100.0);
            let view_proj = camera.view_proj(&viewport);
            let Extent { center, half } = staged.extent();
            let (right, up) = plane_axes(staged.shot());
            for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let p = center + right * half.x * sx + up * half.y * sy;
                let ndc = view_proj.project_point3(p);
                assert!(ndc.x.abs() <= 1.0 && ndc.y.abs() <= 1.0, "{id} {aspect}: {ndc}");
                assert!((0.0..=1.0).contains(&ndc.z), "{id}: profondeur {ndc}");
            }
        }
    }
}
