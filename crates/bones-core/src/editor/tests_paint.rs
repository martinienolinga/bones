//! Tests du mode « Weight Paint » au niveau de l'éditeur : gestes, historique,
//! pipette, et protection des poids peints lors d'une édition du squelette.

use super::*;
use glam::Vec2;

use crate::weight_paint::PaintTool;
use crate::{Skin, Viewport, WeightsTool, head_tail};

fn paint_editor() -> (Editor, Viewport) {
    let mut e = Editor::new(Rig::demo());
    e.camera.yaw = 0.0;
    e.camera.pitch = 0.0;
    e.set_mode(Mode::WeightPaint);
    (e, Viewport::new(1000.0, 800.0))
}

/// Point écran au milieu d'un os (donc sur le maillage).
fn over_bone(e: &Editor, name: &str, vp: &Viewport) -> Vec2 {
    let id = e.rig.skeleton.find(name).unwrap();
    let w = e.display_world();
    let (h, t) = head_tail(&w[id.index()], e.rig.skeleton.bone(id).length);
    e.camera.project(h.lerp(t, 0.5), vp).unwrap()
}

fn select(e: &mut Editor, name: &str) {
    e.select(e.rig.skeleton.find(name));
}

#[test]
fn tapping_a_bone_selects_without_painting() {
    let (mut e, vp) = paint_editor();
    assert!(e.paint_bone().is_none());
    let p = over_bone(&e, "forearm.l", &vp);
    assert!(
        !e.pointer_down(p, &vp, 20.0),
        "sans os sélectionné : pas de trait"
    );
    e.pointer_up();
    assert_eq!(e.selected, e.rig.skeleton.find("forearm.l"));
    assert!(!e.can_undo(), "une sélection ne crée pas d'étape");
    assert!(!e.weights_painted());
}

#[test]
fn a_stroke_paints_and_is_one_undo_step() {
    let (mut e, vp) = paint_editor();
    select(&mut e, "forearm.l");
    let before: Skin = (*e.rig.skin).clone();
    let skin_rev = e.rig.revisions().skin;
    let p = over_bone(&e, "forearm.l", &vp);
    assert!(e.pointer_down(p, &vp, 30.0));
    assert!(e.brush_cursor().is_some(), "curseur visible pendant le trait");
    e.pointer_move(p + Vec2::new(10.0, 0.0), &vp);
    e.pointer_move(p + Vec2::new(20.0, 0.0), &vp);
    e.pointer_up();

    assert!(*e.rig.skin != before, "les poids ont changé");
    assert_ne!(e.rig.revisions().skin, skin_rev, "révision des poids incrémentée");
    assert!(e.weights_painted());
    assert!(e.can_undo());
    e.undo();
    assert_eq!(*e.rig.skin, before, "une seule étape restaure tout");
    assert!(!e.can_undo());
}

#[test]
fn cancelling_a_stroke_restores_everything() {
    let (mut e, vp) = paint_editor();
    select(&mut e, "forearm.l");
    let before: Skin = (*e.rig.skin).clone();
    let p = over_bone(&e, "forearm.l", &vp);
    assert!(e.pointer_down(p, &vp, 30.0));
    e.pointer_move(p + Vec2::new(15.0, 0.0), &vp);
    e.pointer_cancel();
    assert_eq!(*e.rig.skin, before);
    assert!(!e.can_undo());
    assert!(e.brush_cursor().is_none());
}

#[test]
fn sample_tool_sets_brush_weight_without_history() {
    let (mut e, vp) = paint_editor();
    select(&mut e, "forearm.l");
    e.weight_paint.brush.tool = PaintTool::Sample;
    e.weight_paint.brush.weight = 0.0;
    let p = over_bone(&e, "forearm.l", &vp);
    assert!(
        !e.pointer_down(p, &vp, 30.0),
        "la pipette ne démarre pas de trait"
    );
    assert!(
        e.weight_paint.brush.weight > 0.0,
        "poids échantillonné sous le doigt"
    );
    assert!(!e.can_undo());
    assert!(!e.weights_painted());
}

#[test]
fn painted_weights_survive_skeleton_edit_then_automatic_rebinds() {
    let (mut e, vp) = paint_editor();
    select(&mut e, "forearm.l");
    let p = over_bone(&e, "forearm.l", &vp);
    e.pointer_down(p, &vp, 30.0);
    e.pointer_move(p + Vec2::new(12.0, 0.0), &vp);
    e.pointer_up();
    assert!(e.weights_painted());
    let painted: Skin = (*e.rig.skin).clone();

    // Édition du squelette : aucune liaison automatique n'est relancée.
    e.set_mode(Mode::Edit);
    select(&mut e, "forearm.l");
    e.set_selected_length(0.42);
    assert!(e.rig.needs_bind(), "le squelette a changé");
    assert!(
        e.bind_job().is_none(),
        "mais les poids peints ne sont pas recalculés"
    );
    assert_eq!(*e.rig.skin, painted, "les poids restent ceux peints");

    // « Automatic » efface le drapeau et relance la liaison en tâche de fond.
    e.apply_weights_tool(WeightsTool::Automatic);
    assert!(!e.weights_painted());
    let job = e.bind_job().expect("une nouvelle liaison est planifiée");
    let (skin, key) = job.run();
    e.finish_bind(skin, key);
    assert!(!e.rig.needs_bind());
    assert!(*e.rig.skin != painted, "les poids ont été recalculés");
}

#[test]
fn weights_tool_without_selection_reports_and_does_nothing() {
    let (mut e, _vp) = paint_editor();
    e.select(None);
    e.apply_weights_tool(WeightsTool::Invert);
    assert!(!e.weights_painted());
    assert!(!e.can_undo());
    assert!(e.status.is_some());
}

#[test]
fn a_weights_tool_makes_one_undo_step() {
    let (mut e, _vp) = paint_editor();
    select(&mut e, "forearm.l");
    let before: Skin = (*e.rig.skin).clone();
    assert!(!e.can_undo());

    e.apply_weights_tool(WeightsTool::Invert);
    assert!(e.weights_painted());
    assert!(*e.rig.skin != before, "« Invert » change les poids");
    assert!(e.can_undo());
    e.undo();
    assert_eq!(*e.rig.skin, before, "une seule étape restaure tout");
    assert!(!e.can_undo());
}
