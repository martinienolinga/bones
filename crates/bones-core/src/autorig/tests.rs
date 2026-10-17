use super::*;
use glam::Vec3;

use crate::Rig;
use crate::{Pose, head_tail};

#[test]
fn generates_symmetric_humanoid() {
    let rig = Rig::demo();
    let markers = Markers::guess(&rig.mesh).unwrap();
    let (s, limbs) = generate(&markers, &AutoRigOptions::default());
    assert_eq!(limbs.len(), 4);
    for name in [
        "root.x",
        "spine_03.x",
        "head.x",
        "hand.l",
        "hand.r",
        "foot.r",
        "index3.l",
        "toes.l",
    ] {
        assert!(s.find(name).is_some(), "{name} manquant");
    }
    let w = s.rest_world();
    let hl = w[s.find("hand.l").unwrap().index()].translation;
    let hr = w[s.find("hand.r").unwrap().index()].translation;
    let cx = markers.root.x;
    assert!((hl.x - cx + (hr.x - cx)).abs() < 1e-4);
    assert!(hl.x > cx);
    for limb in &limbs {
        assert!(limb.is_valid(&s));
    }
}

#[test]
fn ik_limb_holds_target() {
    let rig = Rig::demo();
    let markers = Markers::guess(&rig.mesh).unwrap();
    let (s, mut limbs) = generate(&markers, &AutoRigOptions::default());
    let mut pose = Pose::rest(&s);
    let leg = &mut limbs[1];
    let goal = leg.target + Vec3::new(0.0, 0.25, 0.15);
    leg.target = goal;
    leg.solve(&s, &mut pose);
    let w = pose.world(&s);
    let ankle = head_tail(&w[leg.lower.index()], s.bone(leg.lower).length).1;
    assert!(ankle.abs_diff_eq(goal, 1e-3), "{ankle:?} vs {goal:?}");
    // Le genou plie vers l'avant (+Z).
    assert!(w[leg.lower.index()].translation.z > goal.z - 0.5);
}
