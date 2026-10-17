//! Tests du rig facial : contrat de noms, construction, orientations et
//! estimation des marqueurs sur le mannequin.

use glam::Vec3;

use super::*;
use crate::Skeleton;

/// Marqueurs du visage synthétiques, centrés sur x = 0 (sans dents ni langue).
fn markers() -> FaceMarkers {
    FaceMarkers {
        eye: Vec3::new(0.05, 1.62, 0.09),
        eye_radius: 0.012,
        brow: Vec3::new(0.05, 1.66, 0.10),
        nose_tip: Vec3::new(0.0, 1.60, 0.13),
        lip_top: Vec3::new(0.0, 1.57, 0.12),
        lip_bottom: Vec3::new(0.0, 1.55, 0.12),
        mouth_corner: Vec3::new(0.025, 1.56, 0.11),
        chin: Vec3::new(0.0, 1.53, 0.115),
        jaw: Vec3::new(0.0, 1.61, 0.01),
        ear: Some(Vec3::new(0.09, 1.63, -0.01)),
        teeth_top: None,
        teeth_bottom: None,
        tongue: None,
    }
}

fn head_skeleton() -> (Skeleton, Vec<FaceBone>) {
    let mut s = Skeleton::new();
    s.add_bone_world(
        "head.x",
        None,
        Vec3::new(0.0, 1.46, 0.0),
        Vec3::new(0.0, 1.75, 0.0),
    );
    let created = build(&mut s, &FaceLayout::new(markers(), 0.0));
    (s, created)
}

#[test]
fn names_follow_the_contract() {
    // Contrat de noms (wip/LISEZMOI.md).
    assert!(NAMES.contains(&"jaw.x"));
    for n in [
        "jaw.x",
        "chin.x",
        "tongue_01.x",
        "tongue_02.x",
        "tongue_03.x",
        "teeth_top.x",
        "teeth_bot.x",
        "eye.l",
        "eye.r",
        "eyelid_top.l",
        "eyelid_bot.r",
        "brow_in.l",
        "brow_mid.l",
        "brow_out.r",
        "cheek.l",
        "nose.x",
        "nostril.l",
        "lip_top.x",
        "lip_top.l",
        "lip_bot.x",
        "lip_bot.l",
        "lip_corner.l",
        "ear.r",
    ] {
        assert!(NAMES.contains(&n), "{n} absent du contrat");
        assert_eq!(FaceBone::from_name(n).unwrap().name(), n);
    }
    assert_eq!(
        FaceBone::from_name("eye.l").unwrap().mirrored(),
        Some(FaceBone::EyeR)
    );
    assert_eq!(FaceBone::from_name("nose.x").unwrap().mirrored(), None);
}

#[test]
fn build_creates_bones_with_the_right_parents() {
    let (s, created) = head_skeleton();
    // Les os optionnels absents (langue, dents) ne sont pas créés.
    assert!(!created.contains(&FaceBone::Tongue1));
    assert!(!created.contains(&FaceBone::TeethTop));
    assert!(created.contains(&FaceBone::Jaw) && created.contains(&FaceBone::EarL));

    let parent_name = |n: &str| {
        let id = s.find(n).unwrap();
        s.bone(s.bone(id).parent().unwrap()).name.clone()
    };
    assert_eq!(parent_name("jaw.x"), "head.x");
    assert_eq!(parent_name("chin.x"), "jaw.x");
    assert_eq!(parent_name("lip_bot.l"), "jaw.x");
    assert_eq!(parent_name("lip_top.l"), "head.x");
    assert_eq!(parent_name("eye.l"), "head.x");

    // present() retrouve les os du visage par leur nom.
    let found = present(&s);
    assert!(found.iter().any(|(b, _)| *b == FaceBone::EyeL));
    assert_eq!(found.len(), created.len());
}

#[test]
fn orientations_match_the_contract() {
    let (s, _) = head_skeleton();
    let rot = |n: &str| s.rest_world()[s.find(n).unwrap().index()].rotation;
    let dir = |n: &str| {
        let (h, t) = s.rest_head_tail(s.find(n).unwrap());
        (t - h).normalize()
    };

    // Œil : pointe vers l'avant (+Z).
    assert!(dir("eye.l").abs_diff_eq(Vec3::Z, 1e-3), "{:?}", dir("eye.l"));
    // Os de surface : +Y local = +Z monde, X local = X monde, Z local = −Y monde.
    let r = rot("nose.x");
    assert!((r * Vec3::Y).abs_diff_eq(Vec3::Z, 1e-3));
    assert!((r * Vec3::X).abs_diff_eq(Vec3::X, 1e-3));
    assert!((r * Vec3::Z).abs_diff_eq(-Vec3::Y, 1e-3));
    // Paupière : X local = X monde (une rotation autour de X ouvre/ferme).
    assert!((rot("eyelid_top.l") * Vec3::X).abs_diff_eq(Vec3::X, 1e-3));

    // Symétrie : eye.r est le miroir de eye.l.
    let hl = s.rest_head_tail(s.find("eye.l").unwrap()).0;
    let hr = s.rest_head_tail(s.find("eye.r").unwrap()).0;
    assert!(hr.abs_diff_eq(Vec3::new(-hl.x, hl.y, hl.z), 1e-4));
}

#[test]
fn guess_finds_a_face_on_the_mannequin() {
    let mesh = crate::samples::mannequin_mesh();
    let body = crate::Markers::guess(&mesh).unwrap();
    let face = FaceMarkers::guess(&mesh, &body).expect("visage du mannequin");
    assert!(face.is_valid());
    // Ordre vertical plausible : sourcil au-dessus de l'œil, au-dessus du nez…
    assert!(face.brow.y > face.eye.y);
    assert!(face.eye.y > face.nose_tip.y);
    assert!(face.nose_tip.y > face.lip_top.y);
    assert!(face.lip_top.y > face.lip_bottom.y);
    assert!(face.eye.x > 0.0 && face.eye_radius > 0.0);
    // Le mannequin a des yeux (pièces séparées) : rayon réaliste.
    assert!(
        (0.005..0.03).contains(&face.eye_radius),
        "rayon {}",
        face.eye_radius
    );
}

#[test]
fn no_face_on_a_featureless_head() {
    // La tête lisse du mannequin de démonstration (capsules) n'a pas de visage.
    let demo = crate::Rig::demo();
    let body = crate::Markers::guess(&demo.mesh).unwrap();
    assert!(FaceMarkers::guess(&demo.mesh, &body).is_none());
}
