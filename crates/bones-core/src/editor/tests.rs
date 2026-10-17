use super::*;
use glam::Vec2;

use crate::{BoneId, Viewport, head_tail};

fn front_editor() -> (Editor, Viewport) {
    let mut e = Editor::new(Rig::demo());
    e.camera.yaw = 0.0;
    e.camera.pitch = 0.0;
    (e, Viewport::new(1000.0, 800.0))
}

fn screen_of_tail(e: &Editor, name: &str, vp: &Viewport) -> Vec2 {
    let id = e.rig.skeleton.find(name).unwrap();
    let w = e.display_world();
    let (h, t) = head_tail(&w[id.index()], e.rig.skeleton.bone(id).length);
    e.camera.project(h.lerp(t, 0.8), vp).unwrap()
}

#[test]
fn dragging_a_hand_moves_ik_target_and_undo_restores() {
    let (mut e, vp) = front_editor();
    let before = e.rig.pose.clone();
    let start = screen_of_tail(&e, "forearm.l", &vp);
    assert!(e.pointer_down(start, &vp, 30.0));
    e.pointer_move(start + Vec2::new(-40.0, -60.0), &vp);
    e.pointer_up();
    assert_ne!(e.rig.pose, before);
    e.undo();
    assert_eq!(e.rig.pose, before);
    e.redo();
    assert_ne!(e.rig.pose, before);
}

#[test]
fn moving_root_keeps_ik_feet_planted() {
    let (mut e, vp) = front_editor();
    let foot = e.rig.skeleton.find("foot.l").unwrap();
    let foot_before = e.rig.world()[foot.index()].translation;
    let root = e.rig.skeleton.find("root.x").unwrap();
    let w = e.rig.world();
    let (h, t) = head_tail(&w[root.index()], e.rig.skeleton.bone(root).length);
    let p = e.camera.project(h.lerp(t, 0.5), &vp).unwrap();
    assert!(e.pointer_down(p, &vp, 12.0));
    assert_eq!(e.selected, Some(root));
    e.pointer_move(p + Vec2::new(0.0, 40.0), &vp);
    e.pointer_up();
    let foot_after = e.rig.world()[foot.index()].translation;
    assert!(
        foot_after.abs_diff_eq(foot_before, 1e-3),
        "{foot_before:?} → {foot_after:?}"
    );
    let hips = e.rig.world()[root.index()].translation;
    assert!(hips.y < w[root.index()].translation.y);
}

#[test]
fn keys_capture_pose_edits_and_playback_interpolates() {
    let (mut e, vp) = front_editor();
    e.add_key();
    e.set_frame(20.0);
    let start = screen_of_tail(&e, "forearm.l", &vp);
    e.pointer_down(start, &vp, 30.0);
    e.pointer_move(start + Vec2::new(0.0, -80.0), &vp);
    e.pointer_up();
    e.add_key();
    assert_eq!(e.rig.clip.keys().len(), 2);
    let posed = e.rig.pose.clone();
    e.set_frame(0.0);
    assert_eq!(e.rig.pose, e.rig.clip.keys()[0].pose);
    e.set_frame(20.0);
    assert_eq!(e.rig.pose, posed);
    e.set_frame(10.0);
    assert_ne!(e.rig.pose, posed);
}

#[test]
fn add_bone_tool_extrudes_from_selection() {
    let (mut e, vp) = front_editor();
    e.set_mode(Mode::Edit);
    e.edit_tool = EditTool::AddBone;
    let head = e.rig.skeleton.find("head.x").unwrap();
    e.select(Some(head));
    let n = e.rig.skeleton.len();
    let p = Vec2::new(500.0, 100.0);
    assert!(e.pointer_down(p, &vp, 1.0));
    e.pointer_move(p + Vec2::new(80.0, -50.0), &vp);
    e.pointer_up();
    assert_eq!(e.rig.skeleton.len(), n + 1);
    let new = BoneId(n as u32);
    assert_eq!(e.rig.skeleton.bone(new).parent(), Some(head));
    assert!(e.rig.needs_bind());
    let job = e.bind_job().unwrap();
    assert!(e.bind_job().is_none());
    let (skin, key) = job.run();
    e.finish_bind(skin, key);
    assert!(!e.rig.needs_bind());
}

#[test]
fn autorig_generates_and_binds() {
    let (mut e, _) = front_editor();
    e.set_mode(Mode::AutoRig);
    assert!(e.markers.is_some());
    e.autorig_options.fingers = false;
    e.generate_rig();
    assert_eq!(e.mode, Mode::Pose);
    assert!(e.rig.skeleton.find("index1.l").is_none());
    assert!(e.rig.needs_bind());
}
