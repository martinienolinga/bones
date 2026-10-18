//! Tests du modèle d'objets : hiérarchie, suivi du squelette, parent gardant
//! la place, cycles, duplication et évaluation.

use std::sync::Arc;

use glam::{Quat, Vec3};

use super::{Binding, ObjectError, Parent, SkinMethod};
use crate::mesh::Primitive;
use crate::{BoneId, Rig, Transform};

fn cube(rig: &mut Rig, name: &str, parent: Parent, at: Vec3) -> crate::ObjectId {
    let local = Transform::from_translation(at);
    rig.add_object(name, Arc::new(Primitive::Cube.mesh()), parent, local)
        .unwrap()
}

#[test]
fn world_transform_resolves_the_chain() {
    let mut rig = Rig::demo();
    let a = cube(&mut rig, "a", Parent::World, Vec3::new(1.0, 0.0, 0.0));
    let b = cube(&mut rig, "b", Parent::Object(a), Vec3::new(0.0, 2.0, 0.0));
    let bones = rig.world();
    let wa = rig.object_world(a, &bones).unwrap();
    let wb = rig.object_world(b, &bones).unwrap();
    assert!(wa.translation.abs_diff_eq(Vec3::new(1.0, 0.0, 0.0), 1e-5));
    assert!(wb.translation.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-5));
    // object_worlds (tous d'un coup) donne le même résultat.
    let all = rig.object_worlds(&bones);
    assert!(
        all[rig.object_index(b).unwrap()]
            .translation
            .abs_diff_eq(wb.translation, 1e-5)
    );
}

#[test]
fn bone_parent_follows_the_pose() {
    let mut rig = Rig::demo();
    let hand = rig
        .skeleton
        .find("hand.l")
        .or_else(|| rig.skeleton.find("forearm.l"))
        .unwrap();
    let id = cube(&mut rig, "sword", Parent::Bone(hand), Vec3::ZERO);
    let rest = rig
        .object_world(id, &rig.skeleton.rest_world())
        .unwrap()
        .translation;
    // On tourne la racine : la main bouge, donc l'objet attaché aussi.
    rig.pose.bones[0].rotation = Quat::from_rotation_y(1.0);
    let posed = rig.object_world(id, &rig.world()).unwrap().translation;
    assert!(!posed.abs_diff_eq(rest, 1e-3), "l'objet devrait suivre l'os");
    let bone = rig.world()[hand.index()].translation;
    assert!(posed.abs_diff_eq(bone, 1e-4), "l'objet est à la tête de l'os");
}

#[test]
fn reparent_keeps_world_position() {
    let mut rig = Rig::demo();
    let hand = rig
        .skeleton
        .find("hand.l")
        .or_else(|| rig.skeleton.find("forearm.l"))
        .unwrap();
    let id = cube(&mut rig, "hat", Parent::World, Vec3::new(0.3, 1.8, 0.2));
    let before = rig.object_world(id, &rig.world()).unwrap();
    rig.set_object_parent(id, Parent::Bone(hand), true, &rig.world())
        .unwrap();
    let after = rig.object_world(id, &rig.world()).unwrap();
    assert!(after.translation.abs_diff_eq(before.translation, 1e-4));
    assert!(matches!(rig.object(id).unwrap().parent(), Parent::Bone(_)));
    // Sans « keep transform », la transformation locale est conservée telle
    // quelle (l'objet saute, sa place en monde change).
    let local = rig.object(id).unwrap().local();
    rig.set_object_parent(id, Parent::World, false, &rig.world())
        .unwrap();
    assert_eq!(rig.object(id).unwrap().local(), local);
}

#[test]
fn cycles_are_rejected() {
    let mut rig = Rig::demo();
    let a = cube(&mut rig, "a", Parent::World, Vec3::ZERO);
    let b = cube(&mut rig, "b", Parent::Object(a), Vec3::ZERO);
    let c = cube(&mut rig, "c", Parent::Object(b), Vec3::ZERO);
    // a ne peut pas devenir enfant de c (son descendant).
    assert_eq!(
        rig.set_object_parent(a, Parent::Object(c), false, &rig.world()),
        Err(ObjectError::Cycle)
    );
    // ni de lui-même.
    assert_eq!(
        rig.set_object_parent(a, Parent::Object(a), false, &rig.world()),
        Err(ObjectError::Cycle)
    );
    assert!(rig.object_world(a, &rig.world()).is_some());
}

#[test]
fn deleting_a_bone_keeps_objects_valid_and_in_place() {
    let mut rig = Rig::demo();
    let bone = rig.skeleton.find("forearm.l").unwrap();
    let parent = rig.skeleton.bone(bone).parent();
    let id = cube(&mut rig, "band", Parent::Bone(bone), Vec3::new(0.0, 0.1, 0.0));
    let before = rig.object_world(id, &rig.world()).unwrap().translation;
    rig.remove_bone(bone);
    // L'objet reste valide : son parent est l'os héritier (ou le monde).
    match rig.object(id).unwrap().parent() {
        Parent::Bone(b) => assert!(rig.skeleton.contains(b)),
        Parent::World => {}
        Parent::Object(_) => panic!("parent inattendu"),
    }
    let after = rig.object_world(id, &rig.world()).unwrap().translation;
    assert!(after.abs_diff_eq(before, 1e-3), "{before:?} -> {after:?}");
    assert!(parent.is_none() || !rig.skeleton.is_empty());
}

#[test]
fn set_skeleton_retargets_objects_by_bone_name() {
    let mut rig = Rig::demo();
    let bone = rig.skeleton.find("head.x").unwrap();
    let id = cube(&mut rig, "glasses", Parent::Bone(bone), Vec3::new(0.0, 0.1, 0.1));
    // Nouveau squelette (régénéré) : l'os "head.x" existe toujours.
    let markers = crate::Markers::guess(&rig.mesh).unwrap();
    let (skeleton, limbs) = crate::autorig::generate(&markers, &Default::default());
    rig.set_skeleton(skeleton, limbs);
    match rig.object(id).unwrap().parent() {
        Parent::Bone(b) => assert_eq!(rig.skeleton.bone(b).name, "head.x"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn duplicate_copies_subtree_with_fresh_ids() {
    let mut rig = Rig::demo();
    let a = cube(&mut rig, "a", Parent::World, Vec3::ZERO);
    let b = cube(&mut rig, "b", Parent::Object(a), Vec3::new(0.0, 1.0, 0.0));
    let copies = rig.duplicate_object(a, true).unwrap();
    assert_eq!(copies.len(), 2);
    assert!(!copies.contains(&a) && !copies.contains(&b));
    // La copie de l'enfant pointe vers la copie du parent, pas l'original.
    let child_copy = rig.object(copies[1]).unwrap();
    assert_eq!(child_copy.parent(), Parent::Object(copies[0]));
    assert_eq!(rig.objects().len(), 4);
}

#[test]
fn binding_switch_marks_object_for_weights() {
    let mut rig = Rig::demo();
    let id = cube(&mut rig, "cloth", Parent::World, Vec3::ZERO);
    assert!(!rig.object_needs_bind(id));
    rig.set_object_binding(id, Some(SkinMethod::Heat)).unwrap();
    assert!(matches!(rig.object(id).unwrap().binding(), Binding::Skinned(_)));
    assert!(rig.object_needs_bind(id), "les poids doivent être recalculés");
    assert_eq!(rig.stale_objects(), vec![id]);
    rig.set_object_binding(id, None).unwrap();
    assert!(!rig.object_needs_bind(id));
}

#[test]
fn transform_model_moves_world_objects() {
    let mut rig = Rig::demo();
    let id = cube(&mut rig, "prop", Parent::World, Vec3::new(0.5, 0.0, 0.0));
    rig.transform_model(&Transform {
        translation: Vec3::new(1.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    });
    let p = rig.object_world(id, &rig.world()).unwrap().translation;
    assert!(p.abs_diff_eq(Vec3::new(1.5, 0.0, 0.0), 1e-4));
}

#[test]
fn reorder_and_names_stay_unique() {
    let mut rig = Rig::demo();
    let a = cube(&mut rig, "box", Parent::World, Vec3::ZERO);
    let b = cube(&mut rig, "box", Parent::World, Vec3::ZERO);
    assert_ne!(rig.object(a).unwrap().name, rig.object(b).unwrap().name);
    rig.reorder_object(b, 0).unwrap();
    assert_eq!(rig.objects()[0].id(), b);
    let _ = BoneId(0);
}
