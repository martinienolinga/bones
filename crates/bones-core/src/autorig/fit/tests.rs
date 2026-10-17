use glam::Vec3;

use super::sanity;
use super::surface::Surface;
use crate::autorig::{AutoRigOptions, Layout, build, fit_layout, generate_fitted};
use crate::skinning::distance_to_segment;
use crate::{Markers, Mesh, Rig};

#[test]
fn section_of_capsule_is_one_centered_ring() {
    let mesh = Mesh::capsule(Vec3::new(1.0, 0.0, 2.0), Vec3::new(1.0, 2.0, 2.0), 0.3, 24, 12);
    let s = Surface::new(&mesh);
    let rings = s.section(Vec3::new(0.0, 1.0, 0.0), Vec3::Y);
    assert_eq!(rings.len(), 1);
    assert!(
        rings[0].center.abs_diff_eq(Vec3::new(1.0, 1.0, 2.0), 1e-3),
        "{rings:?}"
    );
    assert!((rings[0].radius() - 0.3).abs() < 0.01, "{}", rings[0].radius());
}

/// Membres du personnage en pose en A (segments des capsules, côté gauche).
const ARM: (Vec3, Vec3, f32) = (Vec3::new(0.16, 1.42, 0.0), Vec3::new(0.5, 0.86, 0.0), 0.045);
const LEG: (Vec3, Vec3, f32) = (Vec3::new(0.1, 0.95, 0.0), Vec3::new(0.17, 0.08, 0.0), 0.065);

/// Personnage fait de capsules, bras à 60° sous l'horizontale, jambes écartées.
fn a_pose() -> Mesh {
    let mut mesh = Mesh::default();
    let c = |a: Vec3, b: Vec3, r: f32| Mesh::capsule(a, b, r, 20, 24);
    mesh.append(&c(Vec3::new(0.0, 0.92, 0.0), Vec3::new(0.0, 1.42, 0.0), 0.16));
    mesh.append(&c(Vec3::new(0.0, 1.58, 0.01), Vec3::new(0.0, 1.74, 0.01), 0.1));
    for x in [1.0f32, -1.0] {
        let m = Vec3::new(x, 1.0, 1.0);
        mesh.append(&c(ARM.0 * m, ARM.1 * m, ARM.2));
        mesh.append(&c(LEG.0 * m, LEG.1 * m, LEG.2));
    }
    mesh
}

fn inside(p: Vec3, (a, b, r): (Vec3, Vec3, f32)) -> bool {
    distance_to_segment(p, a, b) < r
}

#[test]
fn guess_follows_arms_in_a_pose_and_keeps_t_pose() {
    let mesh = a_pose();
    let m = Markers::guess(&mesh).unwrap();
    assert!(m.wrist.y < m.shoulder.y - 0.3, "poignet {:?}", m.wrist);
    assert!(
        distance_to_segment(m.wrist, ARM.0, ARM.1) < 0.1,
        "poignet {:?}",
        m.wrist
    );
    // Pose en T (démo) : poignet à hauteur de la main.
    let demo = Rig::demo();
    let m = Markers::guess(&demo.mesh).unwrap();
    assert!((m.wrist.y - 1.42).abs() < 0.05, "{:?}", m.wrist);
}

#[test]
fn fitted_joints_stay_inside_the_limbs() {
    let mesh = a_pose();
    let markers = Markers::guess(&mesh).unwrap();
    let mut layout = Layout::from_markers(&markers, &AutoRigOptions::default());
    fit_layout(&mut layout, &mesh);
    for (name, p) in [("coude", layout.elbow), ("poignet", layout.wrist)] {
        assert!(inside(p, ARM), "{name} hors du bras : {p}");
    }
    for (name, p) in [
        ("hanche", layout.hip),
        ("genou", layout.knee),
        ("cheville", layout.ankle),
    ] {
        assert!(inside(p, LEG), "{name} hors de la jambe : {p}");
    }
    // Coude en arrière, genou en avant ; doigts dans le prolongement du bras.
    assert!(layout.elbow.z < 0.0 && layout.knee.z > 0.0);
    let dir = (ARM.1 - ARM.0).normalize();
    assert!((layout.fingers[2][3] - layout.wrist).normalize().dot(dir) > 0.9);

    let (skeleton, limbs) = build(&layout, &AutoRigOptions::default());
    assert!(limbs.iter().all(|l| l.is_valid(&skeleton)));
    let w = skeleton.rest_world();
    let hand = |n: &str| w[skeleton.find(n).unwrap().index()].translation;
    assert!(hand("hand.l").abs_diff_eq(hand("hand.r") * Vec3::new(-1.0, 1.0, 1.0), 1e-4));
}

#[test]
fn fitting_the_t_pose_demo_keeps_a_valid_rig() {
    let demo = Rig::demo();
    let markers = Markers::guess(&demo.mesh).unwrap();
    let (skeleton, limbs) = generate_fitted(&markers, &AutoRigOptions::default(), &demo.mesh);
    assert_eq!(skeleton.len(), demo.skeleton.len());
    assert!(limbs.iter().all(|l| l.is_valid(&skeleton)));
    assert!(skeleton.rest_world().iter().all(|t| t.translation.is_finite()));
}

#[test]
fn unrecognized_parts_keep_the_generic_layout() {
    // Une « pilule » sans bras ni jambes : seul le tronc peut être recentré.
    let blob = Mesh::capsule(Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.0, 1.5, 0.0), 0.3, 24, 24);
    let markers = Markers::guess(&blob).unwrap();
    let generic = Layout::from_markers(&markers, &AutoRigOptions::default());
    let mut fitted = generic.clone();
    fit_layout(&mut fitted, &blob);
    let mut expected = generic.clone();
    expected.restore_torso(&fitted);
    assert_eq!(fitted, expected);

    // Sans ajustement demandé : exactement les marqueurs.
    let options = AutoRigOptions {
        fit_to_mesh: false,
        ..Default::default()
    };
    let demo = Rig::demo();
    let markers = Markers::guess(&demo.mesh).unwrap();
    let plain = crate::autorig::generate(&markers, &options).0;
    let same = generate_fitted(&markers, &options, &demo.mesh).0;
    assert_eq!(plain.rest_world(), same.rest_world());
}

#[test]
fn implausible_fits_are_rejected() {
    let mesh = a_pose();
    let markers = Markers::guess(&mesh).unwrap();
    let generic = Layout::from_markers(&markers, &AutoRigOptions::default());
    let mut fitted = generic.clone();
    fit_layout(&mut fitted, &mesh);
    assert!(sanity::arm_ok(&fitted, &generic) && sanity::leg_ok(&fitted, &generic));

    let mut bad = fitted.clone();
    bad.elbow = bad.wrist + (bad.wrist - bad.shoulder);
    assert!(!sanity::arm_ok(&bad, &generic));
    let mut bad = fitted.clone();
    bad.wrist.x = -bad.wrist.x;
    assert!(!sanity::arm_ok(&bad, &generic));
    let mut bad = fitted.clone();
    bad.knee.y = bad.hip.y + 0.1;
    assert!(!sanity::leg_ok(&bad, &generic));
    let mut bad = fitted.clone();
    bad.ankle = Vec3::splat(f32::NAN);
    assert!(!sanity::leg_ok(&bad, &generic));
    let mut bad = fitted;
    bad.fingers[1][3] = bad.fingers[1][2];
    assert!(!sanity::hand_ok(&bad));
}
