//! Gestes multi-doigts rejoués sur un vrai éditeur (le simulateur X11 ne
//! sait pas produire de contacts multiples).

use super::*;
use bones_core::{Rig, head_tail};

const PPP: f32 = 2.0;

fn editor() -> Editor {
    let mut e = Editor::new(Rig::demo());
    e.camera.yaw = 0.0;
    e.camera.pitch = 0.0;
    e
}

fn viewport() -> Viewport {
    Viewport::new(2000.0, 1600.0)
}

/// Contexte de test : pas d'interface sous les doigts.
fn cx(editor: &mut Editor, pencil_required: bool) -> InputContext<'_> {
    InputContext {
        editor,
        viewport: viewport(),
        pick_radius: 20.0 * PPP,
        tap_slop: 12.0 * PPP,
        over_ui: &|_| false,
        pencil_required,
        pixels_per_point: PPP,
    }
}

/// Point écran au milieu d'un os.
fn bone_point(e: &Editor, name: &str) -> Vec2 {
    let id = e.rig.skeleton.find(name).unwrap();
    let (h, t) = head_tail(&e.rig.world()[id.index()], e.rig.skeleton.bone(id).length);
    e.camera.project(h.lerp(t, 0.5), &viewport()).unwrap()
}

const EMPTY: Vec2 = Vec2::new(150.0, 150.0);

#[test]
fn one_finger_on_empty_space_orbits() {
    let (mut e, mut g) = (editor(), Gestures::default());
    g.touch_start(1, EMPTY, false, cx(&mut e, false));
    g.touch_move(1, EMPTY + Vec2::new(200.0, 0.0), cx(&mut e, false));
    // 200 pixels physiques = 100 points : la sensibilité ne dépend pas de l'écran.
    assert!((e.camera.yaw + 100.0 * 0.008).abs() < 1e-4, "{}", e.camera.yaw);
    assert_eq!(g.touch_end(1, false, cx(&mut e, false)), None);
}

#[test]
fn tap_on_empty_space_deselects() {
    let (mut e, mut g) = (editor(), Gestures::default());
    e.select(e.rig.skeleton.find("spine_02.x"));
    g.touch_start(1, EMPTY, false, cx(&mut e, false));
    g.touch_end(1, false, cx(&mut e, false));
    assert_eq!(e.selected, None);
}

#[test]
fn pinch_zooms_in_and_out() {
    let (mut e, mut g) = (editor(), Gestures::default());
    let d0 = e.camera.distance;
    let (a, b) = (Vec2::new(800.0, 800.0), Vec2::new(1000.0, 800.0));
    g.touch_start(1, a, false, cx(&mut e, false));
    g.touch_start(2, b, false, cx(&mut e, false));
    // Les doigts s'écartent de 200 à 300 px : on se rapproche d'un facteur 1,5.
    g.touch_move(2, b + Vec2::new(100.0, 0.0), cx(&mut e, false));
    assert!((e.camera.distance - d0 / 1.5).abs() < 1e-3 * d0);
    // Puis se rapprochent à 150 px : on s'éloigne.
    g.touch_move(2, a + Vec2::new(150.0, 0.0), cx(&mut e, false));
    assert!((e.camera.distance - d0 / 0.75).abs() < 1e-3 * d0);
    assert_eq!(g.touch_end(2, false, cx(&mut e, false)), None);
    assert_eq!(g.touch_end(1, false, cx(&mut e, false)), None);
}

#[test]
fn two_finger_drag_pans_without_zooming() {
    let (mut e, mut g) = (editor(), Gestures::default());
    let (t0, d0) = (e.camera.target, e.camera.distance);
    let (a, b) = (Vec2::new(800.0, 800.0), Vec2::new(1000.0, 800.0));
    g.touch_start(1, a, false, cx(&mut e, false));
    g.touch_start(2, b, false, cx(&mut e, false));
    let shift = Vec2::new(120.0, 0.0);
    g.touch_move(1, a + shift, cx(&mut e, false));
    g.touch_move(2, b + shift, cx(&mut e, false));
    assert!(
        e.camera.target.x < t0.x,
        "le contenu suit les doigts vers la droite"
    );
    // Le premier doigt bouge seul un instant : léger zoom, puis compensé.
    assert!((e.camera.distance - d0).abs() < 1e-3 * d0);
}

#[test]
fn two_finger_tap_undoes_three_finger_tap_redoes() {
    let (mut e, mut g) = (editor(), Gestures::default());
    for fingers in [2u64, 3] {
        for id in 0..fingers {
            g.touch_start(
                id,
                Vec2::new(500.0 + 150.0 * id as f32, 700.0),
                false,
                cx(&mut e, false),
            );
        }
        let mut action = None;
        for id in 0..fingers {
            action = g.touch_end(id, false, cx(&mut e, false)).or(action);
        }
        let expected = if fingers == 2 {
            GestureAction::Undo
        } else {
            GestureAction::Redo
        };
        assert_eq!(action, Some(expected));
    }
}

#[test]
fn finger_navigates_when_task_requires_pencil() {
    let (mut e, mut g) = (editor(), Gestures::default());
    let p = bone_point(&e, "spine_02.x");
    let yaw = e.camera.yaw;

    // Doigt sur un os, tâche réservée au crayon : on tourne la vue.
    g.touch_start(1, p, false, cx(&mut e, true));
    assert!(!e.is_dragging());
    g.touch_move(1, p + Vec2::new(60.0, 0.0), cx(&mut e, true));
    g.touch_end(1, false, cx(&mut e, true));
    assert_ne!(e.camera.yaw, yaw);
    assert_eq!(e.selected, None);

    // Crayon au même endroit : l'os est saisi.
    let p = bone_point(&e, "spine_02.x");
    g.touch_start(2, p, true, cx(&mut e, true));
    assert!(e.is_dragging());
    g.touch_end(2, false, cx(&mut e, true));
    assert_eq!(e.selected, e.rig.skeleton.find("spine_02.x"));
}

#[test]
fn second_finger_cancels_bone_drag() {
    let (mut e, mut g) = (editor(), Gestures::default());
    let before = e.rig.pose.clone();
    let p = bone_point(&e, "forearm.l");
    g.touch_start(1, p, false, cx(&mut e, false));
    g.touch_move(1, p + Vec2::new(0.0, -150.0), cx(&mut e, false));
    assert_ne!(e.rig.pose, before);
    // Un deuxième doigt arrive : la manipulation est annulée, on passe au pincement.
    g.touch_start(2, p + Vec2::new(300.0, 0.0), false, cx(&mut e, false));
    assert_eq!(e.rig.pose, before);
    assert!(!e.is_dragging());
}

#[test]
fn gestures_starting_on_the_interface_are_ignored() {
    let (mut e, mut g) = (editor(), Gestures::default());
    let camera = e.camera;
    let mut context = cx(&mut e, false);
    context.over_ui = &|_| true;
    g.touch_start(1, EMPTY, false, context);
    g.touch_move(1, EMPTY + Vec2::new(300.0, 0.0), cx(&mut e, false));
    g.touch_end(1, false, cx(&mut e, false));
    assert_eq!(e.camera, camera);
}
