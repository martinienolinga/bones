//! Compteur de modifications de l'éditeur (enregistrement automatique).

use glam::Vec2;

use crate::editor::Mode;
use crate::{Editor, Rig, Viewport, head_tail};

fn front_editor() -> (Editor, Viewport) {
    let mut e = Editor::new(Rig::demo());
    e.camera.yaw = 0.0;
    e.camera.pitch = 0.0;
    (e, Viewport::new(1000.0, 800.0))
}

/// Point à l'écran sur un os hors des membres IK (dont les poignées
/// passeraient avant l'os).
fn on_a_bone(e: &Editor, vp: &Viewport) -> Vec2 {
    let id = e
        .rig
        .skeleton
        .ids()
        .find(|b| b.index() > 0 && !e.rig.limbs.iter().any(|l| l.contains(*b)))
        .unwrap();
    let world = e.rig.world();
    let (h, t) = head_tail(&world[id.index()], e.rig.skeleton.bone(id).length);
    e.camera.project(h.lerp(t, 0.5), vp).unwrap()
}

#[test]
fn a_simple_tap_does_not_count() {
    let (mut e, vp) = front_editor();
    let p = on_a_bone(&e, &vp);
    assert!(e.pointer_down(p, &vp, 30.0));
    e.pointer_up();
    assert!(e.selected.is_some());
    assert_eq!(e.edit_count(), 0);
    e.tap_empty();
    assert_eq!(e.edit_count(), 0);
}

#[test]
fn a_drag_counts_once_when_released() {
    let (mut e, vp) = front_editor();
    let p = on_a_bone(&e, &vp);
    assert!(e.pointer_down(p, &vp, 30.0));
    e.pointer_move(p + Vec2::new(-30.0, -40.0), &vp);
    // Pendant le geste, rien n'est encore compté (pas d'enregistrement en
    // plein mouvement).
    assert_eq!(e.edit_count(), 0);
    e.pointer_up();
    assert_eq!(e.edit_count(), 1);
    assert!(e.can_undo());
}

#[test]
fn a_cancelled_drag_does_not_count() {
    let (mut e, vp) = front_editor();
    let p = on_a_bone(&e, &vp);
    assert!(e.pointer_down(p, &vp, 30.0));
    e.pointer_move(p + Vec2::new(40.0, 10.0), &vp);
    e.pointer_cancel();
    assert_eq!(e.edit_count(), 0);
    assert!(!e.can_undo());
}

#[test]
fn edits_undo_and_redo_count_and_never_decrease() {
    let (mut e, vp) = front_editor();
    let mut last = e.edit_count();
    let mut step = |e: &Editor, changed: bool| {
        let now = e.edit_count();
        assert!(now >= last);
        assert_eq!(now > last, changed, "{last} → {now}");
        last = now;
    };
    e.add_key();
    step(&e, true);
    e.set_frame(10.0);
    step(&e, false);
    e.undo();
    step(&e, true);
    e.redo();
    step(&e, true);
    e.undo();
    e.undo();
    step(&e, true);

    // Une modification suivie d'un toucher sans effet : comptée une fois.
    e.add_key();
    step(&e, true);
    let p = on_a_bone(&e, &vp);
    e.pointer_down(p, &vp, 30.0);
    step(&e, false);
    e.pointer_up();
    step(&e, false);
}

#[test]
fn coalesced_numeric_edits_each_count() {
    let (mut e, _) = front_editor();
    e.set_mode(Mode::Edit);
    e.selected = Some(e.rig.skeleton.ids().last().unwrap());
    let length = e.rig.skeleton.bone(e.selected.unwrap()).length;
    e.set_selected_length(length * 1.1);
    assert_eq!(e.edit_count(), 1);
    e.set_selected_length(length * 1.2);
    e.set_selected_length(length * 1.3);
    assert_eq!(e.edit_count(), 3);
    // Une seule étape d'annulation pour les trois retouches.
    e.undo();
    assert!(!e.can_undo());
    assert_eq!(e.edit_count(), 4);
}
