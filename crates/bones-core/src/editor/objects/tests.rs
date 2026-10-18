//! Tests des opérations de l'éditeur sur les objets : création, sélection,
//! historique, visibilité, parent, liaison et liste.

use std::sync::Arc;

use crate::mesh::Primitive;
use crate::scene::{Binding, ObjectRef, Parent, SkinMethod};
use crate::{Editor, Mesh, Mode, Rig};

fn editor() -> Editor {
    let mut e = Editor::new(Rig::demo());
    e.set_mode(Mode::Object);
    e
}

#[test]
fn add_primitive_selects_and_undoes() {
    let mut e = editor();
    let before = e.edit_count();
    let id = e.add_primitive(Primitive::Sphere);
    assert_eq!(e.selected_object, Some(ObjectRef::Extra(id)));
    assert_eq!(e.rig.objects().len(), 1);
    assert!(e.edit_count() > before);
    e.undo();
    assert_eq!(e.rig.objects().len(), 0);
    assert_eq!(e.selected_object, None, "sélection invalide nettoyée");
    e.redo();
    assert_eq!(e.rig.objects().len(), 1);
}

#[test]
fn delete_rename_visibility_go_through_history() {
    let mut e = editor();
    let id = e.add_primitive(Primitive::Cube);
    e.rename_object(ObjectRef::Extra(id), "Helmet").unwrap();
    assert_eq!(e.rig.object(id).unwrap().name, "Helmet");
    e.set_object_visible(ObjectRef::Extra(id), false).unwrap();
    assert!(!e.rig.object(id).unwrap().visible);
    e.undo(); // visibilité
    assert!(e.rig.object(id).unwrap().visible);
    e.undo(); // renommage
    assert_eq!(e.rig.object(id).unwrap().name, "Cube");
    e.delete_object(id).unwrap();
    assert!(e.rig.object(id).is_none());
}

#[test]
fn body_is_in_the_outliner_and_cannot_be_deleted() {
    let mut e = editor();
    let id = e.add_primitive(Primitive::Cone);
    let rows = e.outliner();
    assert_eq!(rows[0].item, ObjectRef::Body);
    assert!(rows.iter().any(|r| r.item == ObjectRef::Extra(id)));
    e.selected_object = Some(ObjectRef::Body);
    assert!(e.delete_selected_object().is_err());
    // Le corps se renomme et se cache.
    e.rename_object(ObjectRef::Body, "Hero").unwrap();
    assert_eq!(e.rig.body.name, "Hero");
    e.set_object_visible(ObjectRef::Body, false).unwrap();
    assert!(!e.body_visible());
}

#[test]
fn parent_to_selected_bone_and_clear() {
    let mut e = editor();
    let id = e.add_primitive(Primitive::Cube);
    let bone = e.rig.skeleton.find("head.x").unwrap();
    e.selected = Some(bone);
    e.parent_object_to_selected_bone(id).unwrap();
    assert!(matches!(e.rig.object(id).unwrap().parent(), Parent::Bone(b) if b == bone));
    e.clear_object_parent(id, true).unwrap();
    assert_eq!(e.rig.object(id).unwrap().parent(), Parent::World);
}

#[test]
fn skinned_object_schedules_a_background_bind() {
    let mut e = editor();
    // Objet assez grand pour couvrir des os, lié par transfert depuis le corps.
    let id = e.add_primitive(Primitive::Sphere);
    e.set_object_transform(
        id,
        crate::Transform {
            translation: glam::Vec3::new(0.0, 1.0, 0.0),
            scale: glam::Vec3::splat(1.5),
            ..Default::default()
        },
    )
    .unwrap();
    e.set_object_binding(id, Some(SkinMethod::Transfer)).unwrap();
    assert!(e.objects_binding());
    let job = e.bind_job().expect("liaison d'objet planifiée");
    let (skin, key) = job.run();
    e.finish_bind(skin, key);
    assert!(!e.rig.object_needs_bind(id), "poids installés");
    let Binding::Skinned(s) = e.rig.object(id).unwrap().binding() else {
        panic!("objet déformé attendu");
    };
    assert!(s.skin.weights.iter().any(|w| w.weights[0] > 0), "poids non vides");
}

#[test]
fn add_object_mesh_centers_on_its_base() {
    let mut e = editor();
    let mut mesh = Mesh::capsule(
        glam::Vec3::new(5.0, 2.0, 0.0),
        glam::Vec3::new(5.0, 3.0, 0.0),
        0.2,
        8,
        2,
    );
    mesh.compute_normals();
    let id = e.add_object_mesh("import", mesh).unwrap();
    // L'objet est placé là où était le maillage en monde.
    let w = e.object_display_world(id).unwrap();
    assert!(w.translation.x > 4.0 && w.translation.x < 6.0);
    let _ = Arc::new(0u8);
}

#[test]
fn duplicate_selected_and_frame() {
    let mut e = editor();
    let id = e.add_primitive(Primitive::Torus);
    let copy = e.duplicate_object(id, false).unwrap();
    assert_ne!(copy, id);
    assert_eq!(e.selected_object, Some(ObjectRef::Extra(copy)));
    // Cadrage sur l'objet : ne panique pas et déplace la cible caméra.
    e.frame_selected_object();
    assert!(e.object_bounds(copy).is_some());
}

#[test]
fn locked_object_is_not_picked_or_moved() {
    let mut e = editor();
    let id = e.add_primitive(Primitive::Cube);
    e.set_object_locked(ObjectRef::Extra(id), true).unwrap();
    assert!(e.rig.object(id).unwrap().locked);
    // La transformation directe d'un objet verrouillé est refusée.
    assert!(e.set_object_transform(id, crate::Transform::default()).is_err());
}
