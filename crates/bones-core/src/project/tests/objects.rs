//! Objets supplémentaires dans les projets `.kbrig` : aller-retour complet,
//! compatibilité des anciens fichiers, et fichiers hostiles (jamais de
//! panique ni d'allocation démesurée).

use std::sync::Arc;

use serde_json::json;

use super::{TempDir, edit_document, full_editor, save};
use crate::mesh::Primitive;
use crate::project::{ProjectError, load};
use crate::scene::{Binding, ObjectRef, Parent};
use crate::{Editor, Rig, Transform, VertexWeights};

/// Éditeur avec deux objets (un rigide, un déformé à poids manuels).
fn with_objects() -> Editor {
    let e = full_editor();
    assert_eq!(e.rig.objects().len(), 2);
    e
}

#[test]
fn objects_round_trip_through_kbrig() {
    let dir = TempDir::new("objects");
    let editor = with_objects();
    let path = save(&editor, &dir, "Scene");
    let rig = load(&path).unwrap().snapshot.rig;
    assert_eq!(rig.objects().len(), 2);
    for (before, after) in editor.rig.objects().iter().zip(rig.objects()) {
        assert_eq!(before.name, after.name);
        assert_eq!(before.parent(), after.parent());
        assert_eq!(before.local(), after.local());
        assert_eq!(before.mesh().positions, after.mesh().positions);
        assert_eq!(before.mesh().indices, after.mesh().indices);
        assert_eq!(before.mesh().materials.len(), after.mesh().materials.len());
        match (before.binding(), after.binding()) {
            (Binding::Skinned(a), Binding::Skinned(b)) => assert_eq!(a.skin.weights, b.skin.weights),
            (Binding::Rigid, Binding::Rigid) => {}
            _ => panic!("liaison différente"),
        }
    }
    // Le corps conserve son nom et son verrou.
    assert_eq!(rig.body.name, "Hero body");
    assert!(rig.body.locked);
    // Les poids manuels de l'objet déformé sont considérés à jour (pas de
    // recalcul à l'ouverture).
    let editor = load(&path).unwrap().snapshot.into_editor();
    let skinned = editor
        .rig
        .objects()
        .iter()
        .find(|o| o.binding().is_skinned())
        .unwrap();
    assert!(!editor.rig.object_needs_bind(skinned.id()));
}

#[test]
fn old_projects_without_objects_still_load() {
    let dir = TempDir::new("legacy");
    let path = save(&Editor::new(Rig::demo()), &dir, "Legacy");
    // Simule un fichier plus ancien : ni « objects » ni « body ».
    edit_document(&path, |doc| {
        let obj = doc.as_object_mut().unwrap();
        obj.remove("objects");
        doc["rig"].as_object_mut().unwrap().remove("objects");
        doc["rig"].as_object_mut().unwrap().remove("body");
    });
    let rig = load(&path).unwrap().snapshot.rig;
    assert!(rig.objects().is_empty());
    assert_eq!(rig.body.name, "Body");
    assert!(rig.body.visible && !rig.body.locked);
}

#[test]
fn out_of_range_object_indices_are_rejected() {
    let dir = TempDir::new("obj-indices");
    let mut editor = Editor::new(Rig::demo());
    let mut mesh = Primitive::Cube.mesh();
    let bad = mesh.vertex_count() as u32 + 50;
    mesh.indices[0] = bad;
    editor
        .rig
        .add_object("broken", Arc::new(mesh), Parent::World, Transform::IDENTITY)
        .unwrap();
    let path = save(&editor, &dir, "Broken");
    assert!(matches!(load(&path), Err(ProjectError::Invalid(_))));
}

#[test]
fn oversized_object_counts_are_refused() {
    let dir = TempDir::new("obj-huge");
    let path = save(&with_objects(), &dir, "Huge");
    edit_document(&path, |doc| doc["objects"][0]["positions"] = json!(u32::MAX));
    assert!(matches!(load(&path), Err(ProjectError::TooLarge(_))));
}

#[test]
fn bad_object_parents_and_cycles_are_repaired() {
    let dir = TempDir::new("obj-parents");
    let path = save(&with_objects(), &dir, "Parents");
    // Parent sur un os inexistant, et un cycle entre les deux objets.
    edit_document(&path, |doc| {
        let objs = doc["rig"]["objects"].as_array().unwrap();
        let (a, b) = (objs[0]["id"].clone(), objs[1]["id"].clone());
        doc["rig"]["objects"][0]["parent"] = json!({ "Object": b });
        doc["rig"]["objects"][1]["parent"] = json!({ "Object": a });
    });
    let rig = load(&path).unwrap().snapshot.rig;
    assert_eq!(rig.objects().len(), 2);
    // Aucun cycle : tous les mondes se résolvent sans boucle infinie.
    let worlds = rig.object_worlds(&rig.world());
    assert_eq!(worlds.len(), 2);

    let path = save(&with_objects(), &dir, "BadBone");
    edit_document(&path, |doc| {
        doc["rig"]["objects"][0]["parent"] = json!({ "Bone": 9999 })
    });
    let rig = load(&path).unwrap().snapshot.rig;
    assert_eq!(rig.objects()[0].parent(), Parent::World);
}

#[test]
fn duplicate_object_ids_are_reassigned() {
    let dir = TempDir::new("obj-ids");
    let path = save(&with_objects(), &dir, "Ids");
    edit_document(&path, |doc| {
        let id = doc["rig"]["objects"][0]["id"].clone();
        doc["rig"]["objects"][1]["id"] = id;
    });
    let rig = load(&path).unwrap().snapshot.rig;
    assert_eq!(rig.objects().len(), 2);
    assert_ne!(rig.objects()[0].id(), rig.objects()[1].id());
}

#[test]
fn object_weights_with_missing_bones_are_cleared() {
    let dir = TempDir::new("obj-weights");
    let mut editor = Editor::new(Rig::demo());
    let mesh = Primitive::Sphere.mesh();
    let n = mesh.vertex_count();
    let id = editor
        .rig
        .add_object("cloak", Arc::new(mesh), Parent::World, Transform::IDENTITY)
        .unwrap();
    editor
        .rig
        .set_object_weights(
            id,
            crate::Skin {
                weights: vec![VertexWeights::single(crate::BoneId(9999)); n],
            },
        )
        .unwrap();
    let path = save(&editor, &dir, "Weights");
    let rig = load(&path).unwrap().snapshot.rig;
    let o = rig.object(id).unwrap();
    let Binding::Skinned(s) = o.binding() else {
        panic!()
    };
    assert!(
        s.skin
            .weights
            .iter()
            .all(|w| w.iter().all(|(b, _)| rig.skeleton.contains(b)))
    );
    let _ = ObjectRef::Body;
}
