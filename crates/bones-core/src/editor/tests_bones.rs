//! Tests de l'édition complète des os depuis l'éditeur.

use super::*;
use glam::Vec2;

use crate::{Transform, Viewport};

fn edit_editor() -> (Editor, Viewport) {
    let mut e = Editor::new(Rig::demo());
    e.camera.yaw = 0.0;
    e.camera.pitch = 0.0;
    e.set_mode(Mode::Edit);
    (e, Viewport::new(1000.0, 800.0))
}

fn id(e: &Editor, name: &str) -> BoneId {
    e.rig.skeleton.find(name).unwrap()
}

fn screen_mid(e: &Editor, name: &str, vp: &Viewport) -> Vec2 {
    let (h, t) = e.rig.skeleton.rest_head_tail(id(e, name));
    e.camera.project(h.lerp(t, 0.5), vp).unwrap()
}

#[test]
fn reparent_with_reorder_keeps_rig_consistent() {
    let mut rig = Rig::demo();
    let hand = rig.skeleton.find("hand.l").unwrap();
    // Pose, clé, épingle et poids sur la main, repérés par leur nom.
    let delta = Transform::from_rotation_translation(glam::Quat::from_rotation_z(0.4), Vec3::ZERO);
    rig.pose.set(hand, delta);
    let pose = rig.pose.clone();
    rig.clip_mut().set_key(0, pose);
    let tail = rig.skeleton.rest_head_tail(hand).1;
    rig.pins.push(crate::Pin {
        bone: hand,
        target: tail,
    });
    let vertex = (0..rig.skin.weights.len())
        .max_by(|a, b| {
            rig.skin.weights[*a]
                .weight_of(hand)
                .total_cmp(&rig.skin.weights[*b].weight_of(hand))
        })
        .unwrap();
    let weight = rig.skin.weights[vertex].weight_of(hand);
    let limbs: Vec<String> = rig
        .limbs
        .iter()
        .map(|l| rig.skeleton.bone(l.upper).name.clone())
        .collect();

    // La racine (indice 0) devient l'enfant de la tête (indice élevé).
    let (root, head) = (
        rig.skeleton.find("root.x").unwrap(),
        rig.skeleton.find("head.x").unwrap(),
    );
    let remap = rig.set_bone_parent(head, None, false).unwrap();
    let head = remap.get(head).unwrap();
    let root = remap.get(root).unwrap();
    rig.set_bone_parent(root, Some(head), false).unwrap();

    let hand = rig.skeleton.find("hand.l").unwrap();
    assert_eq!(rig.pose.get(hand), delta);
    assert_eq!(rig.clip.keys()[0].pose.get(hand), delta);
    assert_eq!(rig.pins[0].bone, hand);
    assert!((rig.skin.weights[vertex].weight_of(hand) - weight).abs() < 1e-4);
    let after: Vec<String> = rig
        .limbs
        .iter()
        .map(|l| rig.skeleton.bone(l.upper).name.clone())
        .collect();
    assert_eq!(after, limbs);
    assert!(!rig.needs_bind(), "rien n'a bougé : les poids restent liés");
    for (i, b) in rig.skeleton.bones().iter().enumerate() {
        assert!(b.parent().is_none_or(|p| p.index() < i));
    }
}

#[test]
fn reparent_tool_gesture_sequence() {
    let (mut e, vp) = edit_editor();
    e.edit_tool = EditTool::Reparent;
    // Enfant : main gauche ; parent : tête.
    assert!(e.pointer_down(screen_mid(&e, "hand.l", &vp), &vp, 12.0));
    e.pointer_up();
    assert_eq!(e.selected, Some(id(&e, "hand.l")));
    assert!(!e.can_undo(), "choisir l'enfant ne modifie rien");
    assert!(e.pointer_down(screen_mid(&e, "head.x", &vp), &vp, 12.0));
    e.pointer_up();
    let hand = id(&e, "hand.l");
    assert_eq!(e.rig.skeleton.bone(hand).parent(), Some(id(&e, "head.x")));
    assert_eq!(e.status.as_deref(), Some("hand.l → head.x"));
    // Puis la main devient une racine en touchant le vide.
    assert!(e.pointer_down(screen_mid(&e, "hand.l", &vp), &vp, 12.0));
    e.pointer_up();
    e.tap_empty();
    assert!(e.rig.skeleton.bone(id(&e, "hand.l")).parent().is_none());
    // Deux étapes d'annulation, qui restaurent exactement le squelette.
    let original = Rig::demo().skeleton;
    e.undo();
    e.undo();
    assert_eq!(e.rig.skeleton, original);
}

#[test]
fn cycle_is_rejected_without_history_entry() {
    let (mut e, _) = edit_editor();
    let (root, hand) = (id(&e, "root.x"), id(&e, "hand.l"));
    assert!(!e.set_bone_parent(root, Some(hand), false));
    assert!(!e.can_undo());
    assert!(e.status.is_some());
}

#[test]
fn mirror_edit_updates_counterpart() {
    let (mut e, _) = edit_editor();
    e.mirror_edit = true;
    e.select(Some(id(&e, "forearm.l")));
    let (_, tail) = e.rig.skeleton.rest_head_tail(id(&e, "forearm.l"));
    let moved = tail + Vec3::new(0.0, 0.2, 0.1);
    e.set_selected_tail(moved);
    let (_, right) = e.rig.skeleton.rest_head_tail(id(&e, "forearm.r"));
    assert!(
        right.abs_diff_eq(Vec3::new(-moved.x, moved.y, moved.z), 1e-4),
        "{right:?}"
    );
    // Sans symétrie, le côté droit ne bouge plus.
    e.mirror_edit = false;
    e.set_selected_length(0.1);
    let (_, still) = e.rig.skeleton.rest_head_tail(id(&e, "forearm.r"));
    assert!(still.abs_diff_eq(right, 1e-5));
}

#[test]
fn numeric_edits_coalesce_into_one_undo_step() {
    let (mut e, _) = edit_editor();
    let original = e.rig.skeleton.clone();
    e.select(Some(id(&e, "spine_02.x")));
    for roll in [0.1, 0.2, 0.3] {
        e.set_selected_roll(roll);
    }
    assert!((e.rig.skeleton.roll(id(&e, "spine_02.x")) - 0.3).abs() < 1e-4);
    e.undo();
    assert_eq!(e.rig.skeleton, original);
    assert!(!e.can_undo());
}

#[test]
fn duplicate_subdivide_delete_branch() {
    let (mut e, _) = edit_editor();
    let n = e.rig.skeleton.len();
    e.select(Some(id(&e, "forearm.l")));
    e.duplicate_selected(true);
    let copy = e.selected.unwrap();
    assert_eq!(e.rig.skeleton.bone(copy).name, "forearm.001.l");
    let added = e.rig.skeleton.len() - n;
    assert!(added > 1, "la main et les doigts sont copiés");
    assert_eq!(e.rig.pose.bones.len(), e.rig.skeleton.len());
    assert!(e.rig.needs_bind());

    e.select(Some(id(&e, "spine_02.x")));
    e.subdivide_selected(2);
    assert_eq!(e.rig.skeleton.len(), n + added + 1);
    assert_eq!(e.rig.pose.bones.len(), e.rig.skeleton.len());

    // La subdivision a pu renuméroter les os : on retrouve la copie par son nom.
    e.select(e.rig.skeleton.find("forearm.001.l"));
    e.delete_selected_with_children();
    assert_eq!(e.rig.skeleton.len(), n + 1);
    assert!(e.rig.skeleton.find("forearm.001.l").is_none());
}

#[test]
fn move_head_and_bone_tools() {
    let (mut e, vp) = edit_editor();
    let spine = id(&e, "spine_02.x");
    let (h0, t0) = e.rig.skeleton.rest_head_tail(spine);
    e.edit_tool = EditTool::MoveBone;
    let p = screen_mid(&e, "spine_02.x", &vp);
    assert!(e.pointer_down(p, &vp, 10.0));
    e.pointer_move(p + Vec2::new(40.0, 0.0), &vp);
    e.pointer_up();
    let (h1, t1) = e.rig.skeleton.rest_head_tail(spine);
    assert!(h1.x > h0.x && (t1 - h1).abs_diff_eq(t0 - h0, 1e-4));

    e.edit_tool = EditTool::MoveHead;
    let head_screen = e.camera.project(h1, &vp).unwrap();
    assert!(e.pointer_down(
        head_screen.lerp(e.camera.project(t1, &vp).unwrap(), 0.1),
        &vp,
        10.0
    ));
    e.pointer_move(head_screen + Vec2::new(-30.0, 0.0), &vp);
    e.pointer_up();
    let (h2, t2) = e.rig.skeleton.rest_head_tail(spine);
    assert!(t2.abs_diff_eq(t1, 1e-4), "la queue reste en place");
    assert!(h2.x < h1.x);
    // La queue du parent connecté a suivi.
    let parent = e.rig.skeleton.bone(spine).parent().unwrap();
    assert!(e.rig.skeleton.rest_head_tail(parent).1.abs_diff_eq(h2, 1e-4));
}

#[test]
fn deform_flag_excludes_bone_from_weights() {
    let (mut e, _) = edit_editor();
    let spine = id(&e, "spine_02.x");
    e.select(Some(spine));
    e.set_selected_deform(false);
    let job = e.bind_job().unwrap();
    let key = job.key;
    let skin = BindMethod::Envelope.compute(&job.mesh, &job.skeleton);
    assert!(skin.weights.iter().all(|w| w.weight_of(spine) == 0.0));
    e.finish_bind(skin, key);
    assert!(!e.rig.needs_bind());
    let heat = crate::skinning::heat_weights(&e.rig.mesh, &e.rig.skeleton);
    assert!(heat.weights.iter().all(|w| w.weight_of(spine) == 0.0));
}

#[test]
fn selection_navigation() {
    let (mut e, _) = edit_editor();
    e.select(Some(id(&e, "forearm.l")));
    e.select_parent();
    assert_eq!(e.selected, Some(id(&e, "arm.l")));
    e.select_child();
    assert_eq!(e.selected, Some(id(&e, "forearm.l")));
    e.select(Some(id(&e, "thigh.l")));
    e.select_next_sibling();
    let next = e.selected.unwrap();
    assert_ne!(next, id(&e, "thigh.l"));
    assert_eq!(e.rig.skeleton.bone(next).parent(), Some(id(&e, "root.x")));
}
