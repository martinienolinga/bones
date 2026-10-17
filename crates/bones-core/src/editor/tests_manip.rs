//! Manipulateur 3D piloté depuis l'éditeur : os en Pose et en Édition,
//! modèle entier, annulation.

use std::f32::consts::FRAC_PI_2;

use glam::{Quat, Vec2, Vec3};

use super::*;
use crate::manipulator::{Axis, Geometry, Kind, ManipulatorMode, Part};
use crate::{Viewport, head_tail};

fn editor(mode: Mode, manip: ManipulatorMode) -> (Editor, Viewport) {
    let mut e = Editor::new(Rig::demo());
    e.camera.yaw = 0.0;
    e.camera.pitch = 0.0;
    e.set_mode(mode);
    e.manipulator.mode = manip;
    (e, Viewport::new(1000.0, 800.0))
}

fn id(e: &Editor, name: &str) -> BoneId {
    e.rig.skeleton.find(name).unwrap()
}

/// Un point de la partie demandée (centre d'un polygone, premier point
/// d'une boucle, milieu d'un trait).
fn part_point(e: &Editor, vp: &Viewport, part: Part) -> (Vec2, Vec2) {
    let mut g = Geometry::default();
    assert!(e.manipulator_geometry(vp, &mut g));
    let el = g
        .elements()
        .iter()
        .find(|el| el.part == Some(part))
        .expect("partie absente");
    let pts = g.points(el);
    let p = match el.kind {
        Kind::Fill => pts.iter().sum::<Vec2>() / pts.len() as f32,
        Kind::Loop => pts[0],
        Kind::Line => pts[pts.len() / 2],
    };
    (g.center, p)
}

fn drag(e: &mut Editor, vp: &Viewport, path: &[Vec2]) {
    assert!(e.pointer_down(path[0], vp, 20.0));
    assert!(e.manipulator_active().is_some());
    for p in &path[1..] {
        e.pointer_move(*p, vp);
    }
    e.pointer_up();
}

fn same_rotation(a: Quat, b: Quat) -> bool {
    a.abs_diff_eq(b, 2e-3) || a.abs_diff_eq(-b, 2e-3)
}

#[test]
fn pose_view_ring_turns_selected_bone_and_undo_restores() {
    let (mut e, vp) = editor(Mode::Pose, ManipulatorMode::Rotate);
    let spine = id(&e, "spine_02.x");
    e.select(Some(spine));
    let before = e.rig.world()[spine.index()];
    let pose = e.rig.pose.clone();
    let (c, start) = part_point(&e, &vp, Part::ViewRing);
    let r = start.distance(c);
    drag(
        &mut e,
        &vp,
        &[start, c + Vec2::new(r, -r) * 0.7, c + Vec2::new(0.0, -r)],
    );
    let after = e.rig.world()[spine.index()];
    assert!(
        after.translation.abs_diff_eq(before.translation, 1e-4),
        "la tête reste en place"
    );
    let turn = Quat::from_rotation_z(FRAC_PI_2);
    assert!(same_rotation(after.rotation, turn * before.rotation));
    e.undo();
    assert_eq!(e.rig.pose, pose);
}

#[test]
fn pose_arrow_moves_root_and_tap_leaves_no_history() {
    let (mut e, vp) = editor(Mode::Pose, ManipulatorMode::Translate);
    let root = id(&e, "root.x");
    e.select(Some(root));
    let head0 = e.rig.world()[root.index()].translation;
    let (_, tip) = part_point(&e, &vp, Part::Axis(Axis::X));

    // Simple toucher : rien ne change, pas d'étape d'annulation.
    assert!(e.pointer_down(tip, &vp, 20.0));
    e.pointer_up();
    assert!(!e.can_undo());

    drag(
        &mut e,
        &vp,
        &[tip, tip + Vec2::new(50.0, 0.0), tip + Vec2::new(100.0, 0.0)],
    );
    let head1 = e.rig.world()[root.index()].translation;
    assert!(head1.x - head0.x > 0.05, "{head0} → {head1}");
    assert!((head1.y - head0.y).abs() < 1e-4 && (head1.z - head0.z).abs() < 1e-4);
    assert!(e.can_undo());
}

#[test]
fn pose_drag_updates_existing_key() {
    let (mut e, vp) = editor(Mode::Pose, ManipulatorMode::Rotate);
    e.add_key();
    let head = id(&e, "head.x");
    e.select(Some(head));
    let (c, start) = part_point(&e, &vp, Part::ViewRing);
    drag(&mut e, &vp, &[start, c + (start - c).perp()]);
    assert_eq!(e.rig.clip.key_at(0).unwrap().pose, e.rig.pose);
}

#[test]
fn edit_ring_rotates_bone_about_its_head() {
    let (mut e, vp) = editor(Mode::Edit, ManipulatorMode::Rotate);
    let forearm = id(&e, "forearm.l");
    e.select(Some(forearm));
    let (h0, t0) = e.rig.skeleton.rest_head_tail(forearm);
    let (c, start) = part_point(&e, &vp, Part::ViewRing);
    let r = start.distance(c);
    drag(
        &mut e,
        &vp,
        &[start, c + Vec2::new(r, -r) * 0.7, c + Vec2::new(0.0, -r)],
    );
    let (h1, t1) = e.rig.skeleton.rest_head_tail(forearm);
    assert!(h1.abs_diff_eq(h0, 1e-4));
    let expected = h0 + Quat::from_rotation_z(FRAC_PI_2) * (t0 - h0);
    assert!(t1.abs_diff_eq(expected, 1e-3), "{t1} ≠ {expected}");
    // La main connectée suit par sa tête.
    let hand = id(&e, "hand.l");
    assert!(e.rig.skeleton.rest_head_tail(hand).0.abs_diff_eq(t1, 1e-3));
}

#[test]
fn edit_without_selection_moves_whole_model() {
    let (mut e, vp) = editor(Mode::Edit, ManipulatorMode::Translate);
    e.select(None);
    let (lo0, hi0) = e.rig.mesh.bounds().unwrap();
    let root = id(&e, "root.x");
    let root0 = e.rig.skeleton.rest_head_tail(root).0;
    let (_, tip) = part_point(&e, &vp, Part::Axis(Axis::X));
    drag(&mut e, &vp, &[tip, tip + Vec2::new(80.0, 0.0)]);
    let (lo1, hi1) = e.rig.mesh.bounds().unwrap();
    let shift = lo1 - lo0;
    assert!(shift.x > 0.05 && shift.y.abs() < 1e-4 && shift.z.abs() < 1e-4);
    assert!((hi1 - hi0).abs_diff_eq(shift, 1e-4));
    let root1 = e.rig.skeleton.rest_head_tail(root).0;
    assert!((root1 - root0).abs_diff_eq(shift, 1e-4));
    assert!(!e.rig.needs_bind(), "les poids restent valables");
    e.undo();
    assert_eq!(e.rig.mesh.bounds().unwrap(), (lo0, hi0));
}

#[test]
fn model_transform_keeps_rolls_and_scales_animation() {
    let mut rig = Rig::demo();
    let root = rig.skeleton.find("root.x").unwrap();
    let mut pose = rig.pose.clone();
    pose.bones[root.index()].translation = Vec3::new(0.0, 0.0, 0.3);
    rig.pose = pose.clone();
    rig.clip_mut().set_key(0, pose);
    let hand = rig.skeleton.find("hand.l").unwrap();
    let before = rig.skeleton.rest_world();
    let (h0, t0) = head_tail(&before[hand.index()], rig.skeleton.bone(hand).length);
    let target0 = rig.limbs[0].target;

    let turn = Quat::from_rotation_y(FRAC_PI_2);
    let m = crate::Transform {
        translation: Vec3::new(1.0, 0.0, 0.0),
        rotation: turn,
        scale: Vec3::splat(2.0),
    };
    rig.transform_model(&m);
    let after = rig.skeleton.rest_world();
    assert!(same_rotation(
        after[hand.index()].rotation,
        turn * before[hand.index()].rotation
    ));
    let (h1, t1) = head_tail(&after[hand.index()], rig.skeleton.bone(hand).length);
    assert!(h1.abs_diff_eq(m.transform_point(h0), 1e-4));
    assert!(t1.abs_diff_eq(m.transform_point(t0), 1e-4));
    assert!(rig.limbs[0].target.abs_diff_eq(m.transform_point(target0), 1e-4));
    // Le déplacement animé de la racine double avec le modèle.
    let posed = rig.world()[root.index()].translation - after[root.index()].translation;
    assert!((posed.length() - 0.6).abs() < 1e-4, "{posed}");
    assert_eq!(rig.clip.keys()[0].pose, rig.pose);
    assert!(!rig.needs_bind());
}
