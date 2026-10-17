use super::*;
use glam::Vec3;

use crate::Rig;
use crate::{Mesh, Pose, head_tail};

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

#[test]
fn generate_safe_matches_fitted_on_a_sound_mesh() {
    // Sur le mannequin de démonstration, l'ajustement est sain : le garde-fou
    // laisse passer le résultat recalé.
    let demo = Rig::demo();
    let markers = Markers::guess(&demo.mesh).unwrap();
    let opts = AutoRigOptions::default();
    let safe = generate_safe(&markers, &opts, &demo.mesh).0;
    let fitted = generate_fitted(&markers, &opts, &demo.mesh).0;
    assert_eq!(safe.rest_world(), fitted.rest_world());
}

#[test]
fn generate_safe_never_panics_on_a_degenerate_mesh() {
    // Marqueurs plausibles mais maillage dégénéré (sommets confondus) : pas de
    // panique, squelette fini et membres valides (repli sur les marqueurs).
    let markers = Markers::guess(&Rig::demo().mesh).unwrap();
    let bad = Mesh {
        positions: vec![Vec3::ZERO; 30],
        indices: (0..30).collect(),
        ..Default::default()
    };
    let (s, limbs) = generate_safe(&markers, &AutoRigOptions::default(), &bad);
    assert!(!s.is_empty() && limbs.len() == 4);
    assert!(s.rest_world().iter().all(|t| t.translation.is_finite()));
    assert!(s.bones().iter().all(|b| b.length > 0.0));
}
