use super::*;
use std::sync::Arc;

#[test]
fn demo_is_bound_and_roundtrips() {
    let rig = Rig::demo();
    assert!(!rig.needs_bind());
    assert!(rig.skeleton.len() > 20);
    let mut bytes = Vec::new();
    rig.save(&mut bytes).unwrap();
    let back = Rig::load(bytes.as_slice()).unwrap();
    assert_eq!(back.skeleton, rig.skeleton);
    assert_eq!(*back.skin, *rig.skin);
    assert_eq!(back.limbs.len(), 4);
    assert!(!back.needs_bind());
}

#[test]
fn clone_shares_heavy_data() {
    let rig = Rig::demo();
    let copy = rig.clone();
    assert!(Arc::ptr_eq(&rig.mesh, &copy.mesh));
    assert!(Arc::ptr_eq(&rig.skin, &copy.skin));
}

#[test]
fn remove_bone_keeps_everything_consistent() {
    let mut rig = Rig::demo();
    let pose = rig.pose.clone();
    rig.clip_mut().set_key(0, pose);
    let hand = rig.skeleton.find("hand.l").unwrap();
    rig.remove_bone(hand);
    assert!(rig.skeleton.find("hand.l").is_none());
    assert_eq!(rig.pose.bones.len(), rig.skeleton.len());
    assert_eq!(rig.clip.keys()[0].pose.bones.len(), rig.skeleton.len());
    assert!(rig.limbs.iter().all(|l| l.is_valid(&rig.skeleton)));
    assert_eq!(rig.limbs.len(), 3);
    assert!(!rig.needs_bind());
}

#[test]
fn stale_bind_result_is_rejected() {
    let mut rig = Rig::demo();
    rig.touch_skeleton();
    let stale = (0, 0);
    assert!(!rig.apply_skin((*rig.skin).clone(), stale));
    assert!(rig.needs_bind());
}
