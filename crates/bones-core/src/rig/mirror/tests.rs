use super::*;
use crate::head_tail;

fn arms() -> Skeleton {
    let mut s = Skeleton::new();
    let spine = s.add_bone_world("spine.x", None, Vec3::ZERO, Vec3::Y);
    let al = s.add_bone_world(
        "arm.l",
        Some(spine),
        Vec3::new(0.2, 1.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
    );
    s.add_bone_world(
        "forearm.l",
        Some(al),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(1.8, 1.0, 0.0),
    );
    s
}

#[test]
fn names() {
    assert_eq!(mirror_name("arm.l").as_deref(), Some("arm.r"));
    assert_eq!(mirror_name("Hand_R").as_deref(), Some("Hand_L"));
    assert_eq!(
        mirror_name("mixamorig:LeftArm").as_deref(),
        Some("mixamorig:RightArm")
    );
    assert_eq!(mirror_name("spine.x"), None);
    assert_eq!(side_of("thigh.r"), Some(Side::Right));
}

#[test]
fn symmetrize_creates_right_side() {
    let mut s = arms();
    assert_eq!(symmetrize(&mut s, Side::Left), 2);
    let fr = s.find("forearm.r").unwrap();
    assert_eq!(s.bone(fr).parent(), s.find("arm.r"));
    let w = s.rest_world();
    let (h, t) = head_tail(&w[fr.index()], s.bone(fr).length);
    assert!(h.abs_diff_eq(Vec3::new(-1.0, 1.0, 0.0), 1e-5));
    assert!(t.abs_diff_eq(Vec3::new(-1.8, 1.0, 0.0), 1e-5));
}

#[test]
fn mirrored_pose_swaps_sides() {
    let mut s = arms();
    symmetrize(&mut s, Side::Left);
    let al = s.find("arm.l").unwrap();
    let fr = s.find("forearm.r").unwrap();
    let fl = s.find("forearm.l").unwrap();

    // Rest → rest.
    let rest = Pose::rest(&s);
    let m = mirrored_pose(&s, &rest);
    for t in &m.bones {
        assert!(t.abs_diff_eq(&Transform::IDENTITY, 1e-4));
    }

    // Le bras gauche levé devient le bras droit levé.
    let mut pose = Pose::rest(&s);
    let parent = pose.world(&s)[s.bone(al).parent().unwrap().index()];
    pose.rotate_world(&s, al, &parent, Quat::from_rotation_z(0.8));
    let left_tail = head_tail(&pose.world(&s)[fl.index()], s.bone(fl).length).1;
    let mirrored = mirrored_pose(&s, &pose);
    let right_tail = head_tail(&mirrored.world(&s)[fr.index()], s.bone(fr).length).1;
    assert!(
        right_tail.abs_diff_eq(mirror_point(left_tail), 1e-4),
        "{right_tail:?} vs {left_tail:?}"
    );
}
