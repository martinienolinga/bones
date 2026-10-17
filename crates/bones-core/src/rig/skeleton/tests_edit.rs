//! Tests des opérations d'édition : géométrie, hiérarchie, copies.

use super::*;

/// a (0→1 sur Y) → b (1→2) → c (horizontal), tous connectés, plus d
/// rattaché à a sans connexion.
fn rig() -> Skeleton {
    let mut s = Skeleton::new();
    let a = s.add_bone_world("a", None, Vec3::ZERO, Vec3::Y);
    let b = s.add_bone_world("b", Some(a), Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
    s.add_bone_world("c", Some(b), Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 2.0, 0.0));
    s.add_bone_world("d.l", Some(a), Vec3::new(0.5, 0.5, 0.0), Vec3::new(1.5, 0.5, 0.0));
    s
}

fn ht(s: &Skeleton, name: &str) -> (Vec3, Vec3) {
    s.rest_head_tail(s.find(name).unwrap())
}

fn assert_valid_order(s: &Skeleton) {
    for (i, b) in s.bones().iter().enumerate() {
        assert!(b.parent().is_none_or(|p| p.index() < i), "ordre invalide en {i}");
    }
}

#[test]
fn head_move_drags_connected_parent_tail() {
    let mut s = rig();
    let b = s.find("b").unwrap();
    let p = Vec3::new(0.3, 1.2, 0.0);
    s.set_head_world(b, p);
    assert!(ht(&s, "a").1.abs_diff_eq(p, 1e-4), "la queue du parent suit");
    assert!(ht(&s, "b").0.abs_diff_eq(p, 1e-4));
    assert!(
        ht(&s, "b").1.abs_diff_eq(Vec3::new(0.0, 2.0, 0.0), 1e-4),
        "la queue reste"
    );
    assert!(s.is_connected(b));
    // L'os non connecté ne bouge pas.
    assert!(ht(&s, "d.l").0.abs_diff_eq(Vec3::new(0.5, 0.5, 0.0), 1e-4));
}

#[test]
fn translate_moves_bone_and_its_joints() {
    let mut s = rig();
    let delta = Vec3::new(0.0, 0.0, 0.5);
    s.translate_world(s.find("b").unwrap(), delta);
    assert!(ht(&s, "b").0.abs_diff_eq(Vec3::new(0.0, 1.0, 0.5), 1e-4));
    assert!(ht(&s, "b").1.abs_diff_eq(Vec3::new(0.0, 2.0, 0.5), 1e-4));
    assert!(ht(&s, "a").1.abs_diff_eq(Vec3::new(0.0, 1.0, 0.5), 1e-4));
    assert!(ht(&s, "c").0.abs_diff_eq(Vec3::new(0.0, 2.0, 0.5), 1e-4));
    assert!(ht(&s, "c").1.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-4));
}

#[test]
fn roll_roundtrip_keeps_children_in_place() {
    let mut s = rig();
    let b = s.find("b").unwrap();
    let before = ht(&s, "c");
    s.set_roll(b, 0.7);
    assert!((s.roll(b) - 0.7).abs() < 1e-4);
    assert!(ht(&s, "b").1.abs_diff_eq(Vec3::new(0.0, 2.0, 0.0), 1e-4));
    let after = ht(&s, "c");
    assert!(after.0.abs_diff_eq(before.0, 1e-4) && after.1.abs_diff_eq(before.1, 1e-4));
}

#[test]
fn length_align_and_flip() {
    let mut s = rig();
    let d = s.find("d.l").unwrap();
    s.set_length(d, 2.0);
    assert!(ht(&s, "d.l").1.abs_diff_eq(Vec3::new(2.5, 0.5, 0.0), 1e-4));
    s.align(d, Vec3::Z);
    assert!(ht(&s, "d.l").1.abs_diff_eq(Vec3::new(0.5, 0.5, 2.0), 1e-4));
    s.flip(d);
    assert!(ht(&s, "d.l").0.abs_diff_eq(Vec3::new(0.5, 0.5, 2.0), 1e-4));
    assert!(ht(&s, "d.l").1.abs_diff_eq(Vec3::new(0.5, 0.5, 0.0), 1e-4));
}

#[test]
fn connect_and_disconnect() {
    let mut s = rig();
    let d = s.find("d.l").unwrap();
    assert!(!s.is_connected(d));
    s.set_connected(d, true);
    assert!(s.is_connected(d));
    assert!(ht(&s, "d.l").0.abs_diff_eq(Vec3::Y, 1e-4));
    assert!(ht(&s, "d.l").1.abs_diff_eq(Vec3::new(1.5, 0.5, 0.0), 1e-4));
    // Déconnecté : déplacer la tête n'entraîne plus le parent.
    s.set_connected(d, false);
    s.set_head_world(d, Vec3::new(0.2, 0.2, 0.0));
    assert!(ht(&s, "a").1.abs_diff_eq(Vec3::Y, 1e-4));
}

#[test]
fn reparent_reorders_and_keeps_world() {
    let mut s = rig();
    let before: Vec<(Vec3, Vec3)> = s.ids().map(|id| s.rest_head_tail(id)).collect();
    // a (indice 0) devient enfant de c (indice 2) : impossible (cycle).
    let (a, c) = (s.find("a").unwrap(), s.find("c").unwrap());
    assert_eq!(s.set_parent(a, Some(c), false).err(), Some(SkeletonError::Cycle));
    // b devient enfant de d.l (indice plus grand) : réordonnancement.
    let (b, d) = (s.find("b").unwrap(), s.find("d.l").unwrap());
    let remap = s.set_parent(b, Some(d), false).unwrap();
    assert_valid_order(&s);
    let nb = remap.get(b).unwrap();
    assert_eq!(s.bone(nb).parent(), remap.get(d));
    for (old, (h, t)) in before.iter().enumerate() {
        let new = remap.get(BoneId(old as u32)).unwrap();
        let (h2, t2) = s.rest_head_tail(new);
        assert!(
            h2.abs_diff_eq(*h, 1e-4) && t2.abs_diff_eq(*t, 1e-4),
            "os {old} déplacé"
        );
    }
    // Racine : plus de parent.
    let c = s.find("c").unwrap();
    let remap = s.set_parent(c, None, false).unwrap();
    assert!(s.bone(remap.get(c).unwrap()).parent().is_none());
    assert_valid_order(&s);
}

#[test]
fn reparent_with_connection_moves_head() {
    let mut s = rig();
    let (c, d) = (s.find("c").unwrap(), s.find("d.l").unwrap());
    s.set_parent(d, Some(c), true).unwrap();
    let d = s.find("d.l").unwrap();
    assert!(s.is_connected(d));
    assert!(ht(&s, "d.l").0.abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-4));
}

#[test]
fn duplicate_keeps_side_and_hierarchy() {
    let mut s = rig();
    let b = s.find("b").unwrap();
    let copies = s.duplicate(b, true);
    assert_eq!(copies.len(), 2);
    assert_eq!(s.bone(copies[0]).name, "b.001");
    assert_eq!(s.bone(copies[1]).parent(), Some(copies[0]));
    assert_eq!(s.bone(copies[0]).parent(), s.find("a"));
    assert!(
        s.rest_head_tail(copies[1])
            .1
            .abs_diff_eq(Vec3::new(1.0, 2.0, 0.0), 1e-4)
    );
    let d = s.find("d.l").unwrap();
    let copy = s.duplicate(d, false)[0];
    assert_eq!(s.bone(copy).name, "d.001.l");
    assert_eq!(s.copy_name("d.001.l"), "d.002.l");
}

#[test]
fn subdivide_makes_connected_chain() {
    let mut s = rig();
    let b = s.find("b").unwrap();
    let (pieces, _) = s.subdivide(b, 3);
    assert_eq!(pieces.len(), 3);
    assert_valid_order(&s);
    for w in pieces.windows(2) {
        assert_eq!(s.bone(w[1]).parent(), Some(w[0]));
        assert!(s.is_connected(w[1]));
    }
    assert!(
        s.rest_head_tail(pieces[2])
            .1
            .abs_diff_eq(Vec3::new(0.0, 2.0, 0.0), 1e-4)
    );
    let c = s.find("c").unwrap();
    assert_eq!(s.bone(c).parent(), Some(pieces[2]));
    assert!(s.is_connected(c));
}

#[test]
fn remove_branch() {
    let mut s = rig();
    let remap = s.remove_with_children(s.find("b").unwrap());
    assert_eq!(s.len(), 2);
    assert!(s.find("c").is_none());
    assert_eq!(remap.removed_parent, s.find("a"));
    assert_eq!(remap.get(BoneId(3)), s.find("d.l"));
}
