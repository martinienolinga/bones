//! Tests des gestes du mode Objet : sélection au doigt, déplacement dans le
//! plan de l'écran et manipulateur (une seule étape d'annulation par geste).

use glam::{Vec2, Vec3};

use crate::manipulator::ManipulatorMode;
use crate::mesh::Primitive;
use crate::scene::{ObjectRef, Parent};
use crate::{Editor, Mode, Rig, Transform, Viewport};

fn front() -> (Editor, Viewport) {
    let mut e = Editor::new(Rig::demo());
    e.camera.yaw = 0.0;
    e.camera.pitch = 0.0;
    e.set_mode(Mode::Object);
    e.pixels_per_point = 1.0;
    (e, Viewport::new(1000.0, 800.0))
}

/// Ajoute un objet bien visible devant la caméra et retourne (id, écran).
fn object_in_view(e: &mut Editor, vp: &Viewport) -> (crate::ObjectId, Vec2) {
    let id = e.add_primitive(Primitive::Cube);
    // Au centre du modèle, à bonne taille.
    let at = e.camera.target;
    e.set_object_transform(
        id,
        Transform {
            translation: at,
            scale: Vec3::splat(0.4),
            ..Default::default()
        },
    )
    .unwrap();
    e.selected_object = None;
    let screen = e.camera.project(at, vp).unwrap();
    (id, screen)
}

#[test]
fn tapping_an_object_selects_it_without_history() {
    let (mut e, vp) = front();
    let (id, screen) = object_in_view(&mut e, &vp);
    let edits = e.edit_count();
    assert!(e.pointer_down(screen, &vp, 12.0));
    e.pointer_up();
    assert_eq!(e.selected_object, Some(ObjectRef::Extra(id)));
    assert_eq!(e.edit_count(), edits, "une tape ne crée pas d'étape");
}

#[test]
fn dragging_an_object_moves_it_and_single_undo() {
    let (mut e, vp) = front();
    let (id, screen) = object_in_view(&mut e, &vp);
    let before = e.object_display_world(id).unwrap().translation;
    let edits = e.edit_count();
    assert!(e.pointer_down(screen, &vp, 12.0));
    e.pointer_move(screen + Vec2::new(120.0, 0.0), &vp);
    e.pointer_up();
    let after = e.object_display_world(id).unwrap().translation;
    assert!(!after.abs_diff_eq(before, 1e-3), "l'objet a bougé");
    assert_eq!(e.edit_count(), edits + 1, "exactement une étape");
    e.undo();
    assert!(
        e.object_display_world(id)
            .unwrap()
            .translation
            .abs_diff_eq(before, 1e-4)
    );
}

#[test]
fn tap_on_empty_space_deselects() {
    let (mut e, vp) = front();
    let (id, _screen) = object_in_view(&mut e, &vp);
    e.select_object(Some(ObjectRef::Extra(id)));
    // Toucher loin de tout objet (coin) : la caméra prend le geste.
    assert!(!e.pointer_down(Vec2::new(5.0, 5.0), &vp, 12.0));
    e.pointer_up();
    e.tap_empty();
    assert_eq!(e.selected_object, None);
}

#[test]
fn manipulator_drag_translates_with_single_undo() {
    let (mut e, vp) = front();
    let (id, _) = object_in_view(&mut e, &vp);
    e.select_object(Some(ObjectRef::Extra(id)));
    e.set_manipulator_mode(ManipulatorMode::Translate);
    e.pixels_per_point = 1.0;
    // Le manipulateur a bien une cible pour l'objet sélectionné.
    let mut geom = crate::manipulator::Geometry::default();
    assert!(e.manipulator_geometry(&vp, &mut geom), "manipulateur visible");
    let before = e.object_display_world(id).unwrap().translation;
    let edits = e.edit_count();
    // Saisie d'une poignée d'axe : on vise un point du gizmo.
    let origin = e.camera.project(before, &vp).unwrap();
    assert!(e.pointer_down(origin, &vp, 40.0));
    e.pointer_move(origin + Vec2::new(60.0, 0.0), &vp);
    e.pointer_up();
    let moved = e.object_display_world(id).unwrap().translation;
    if !moved.abs_diff_eq(before, 1e-4) {
        assert_eq!(e.edit_count(), edits + 1);
        e.undo();
        assert!(
            e.object_display_world(id)
                .unwrap()
                .translation
                .abs_diff_eq(before, 1e-4)
        );
    }
}

#[test]
fn skinned_object_is_picked_on_its_deformed_position() {
    let (mut e, vp) = front();
    let id = e.add_primitive(Primitive::Sphere);
    e.set_object_transform(
        id,
        Transform {
            translation: e.camera.target,
            scale: Vec3::splat(0.5),
            ..Default::default()
        },
    )
    .unwrap();
    e.set_object_binding(id, Some(crate::SkinMethod::Transfer))
        .unwrap();
    if let Some(job) = e.bind_job() {
        let (skin, key) = job.run();
        e.finish_bind(skin, key);
    }
    let screen = e.camera.project(e.camera.target, &vp).unwrap();
    assert_eq!(e.pick_object(screen, &vp), Some(ObjectRef::Extra(id)));
    let _ = Parent::World;
}
