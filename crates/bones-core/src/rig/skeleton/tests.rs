use super::*;

fn chain3() -> Skeleton {
    let mut s = Skeleton::new();
    let a = s.add_bone_world("a", None, Vec3::ZERO, Vec3::Y);
    let b = s.add_bone_world("b", Some(a), Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
    s.add_bone_world("c", Some(b), Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 2.0, 0.0));
    s
}

#[test]
fn add_bone_world_roundtrip() {
    let s = chain3();
    let w = s.rest_world();
    let (h, t) = head_tail(&w[2], s.bone(BoneId(2)).length);
    assert!(h.abs_diff_eq(Vec3::new(0.0, 2.0, 0.0), 1e-5));
    assert!(t.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-5));
    assert!(s.is_connected(BoneId(1)));
    assert!(s.is_connected(BoneId(2)));
}

#[test]
fn chain_and_names() {
    let s = chain3();
    assert_eq!(s.chain(BoneId(2), 2), vec![BoneId(1), BoneId(2)]);
    assert_eq!(s.chain(BoneId(2), 10), vec![BoneId(0), BoneId(1), BoneId(2)]);
    assert_eq!(s.unique_name("a"), "a.001");
    assert_eq!(s.unique_name("z"), "z");
    assert_eq!(s.depth(BoneId(2)), 2);
}

#[test]
fn set_tail_keeps_child_tail() {
    let mut s = chain3();
    s.set_tail_world(BoneId(1), Vec3::new(0.5, 2.5, 0.0));
    let w = s.rest_world();
    let (_, t1) = head_tail(&w[1], s.bone(BoneId(1)).length);
    let (h2, t2) = head_tail(&w[2], s.bone(BoneId(2)).length);
    assert!(t1.abs_diff_eq(Vec3::new(0.5, 2.5, 0.0), 1e-4));
    assert!(h2.abs_diff_eq(t1, 1e-4));
    assert!(t2.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-4));
    assert!(s.is_connected(BoneId(2)));
}

#[test]
fn remove_reparents_and_keeps_world() {
    let mut s = chain3();
    let before = s.rest_world()[2];
    let remap = s.remove_bone(BoneId(1));
    assert_eq!(s.len(), 2);
    assert_eq!(remap.get(BoneId(2)), Some(BoneId(1)));
    assert_eq!(remap.get(BoneId(1)), None);
    assert_eq!(remap.removed_parent, Some(BoneId(0)));
    assert_eq!(s.bone(BoneId(1)).parent(), Some(BoneId(0)));
    assert!(s.rest_world()[1].abs_diff_eq(&before, 1e-5));
}

#[test]
fn deserialize_rejects_bad_order() {
    let bones = vec![Bone::new("x", Some(BoneId(0)), Transform::IDENTITY, 1.0)];
    assert!(Skeleton::try_from(bones).is_err());
}
