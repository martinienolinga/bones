//! Tests des expressions faciales : application au visage du mannequin (sans
//! NaN, corps intact), intensité, miroir, superposition additive, écriture de
//! clé, dégradation sur un rig incomplet, et correspondance de noms importés.

use glam::Vec3;

use super::foreign;
use crate::presets::model::{Category, Preset, PresetKind};
use crate::{BoneId, Editor, FaceBone, Mesh, Pose, Rig, Skeleton};

fn is_expr(p: &&Preset) -> bool {
    p.info.category == Category::FacialExpression
}

fn face_preset(id: &str) -> Preset {
    crate::presets::builtin()
        .iter()
        .find(|p| p.info.id == id)
        .unwrap_or_else(|| panic!("{id}"))
        .clone()
}

fn angle(pose: &Pose, skeleton: &Skeleton, name: &str) -> f32 {
    pose.get(skeleton.find(name).unwrap()).rotation.to_axis_angle().1
}

#[test]
fn every_expression_applies_to_the_mannequin_face_only() {
    let base = crate::samples::mannequin();
    let face: Vec<BoneId> = crate::autorig::face::present(&base.skeleton)
        .iter()
        .map(|(_, id)| *id)
        .collect();
    for preset in crate::presets::builtin().iter().filter(is_expr) {
        let mut e = Editor::new(base.clone());
        let before = e.rig.pose.clone();
        e.apply_expression(preset, 1.0, false, false, false).unwrap();
        for (i, (a, b)) in before.bones.iter().zip(&e.rig.pose.bones).enumerate() {
            if !face.contains(&BoneId(i as u32)) {
                assert!(
                    a.abs_diff_eq(b, 1e-6),
                    "corps modifié ({}) os {i}",
                    preset.info.id
                );
            }
            assert!(
                b.translation.is_finite() && b.rotation.is_finite() && b.scale.is_finite(),
                "NaN dans {}",
                preset.info.id
            );
        }
    }
}

#[test]
fn intensity_scales_the_delta() {
    let mut e = Editor::new(crate::samples::mannequin());
    let smile = face_preset("builtin.face.happy");
    e.apply_expression(&smile, 1.0, false, false, false).unwrap();
    let full = angle(&e.rig.pose, &e.rig.skeleton, "lip_corner.l");
    e.undo();
    e.apply_expression(&smile, 0.5, false, false, false).unwrap();
    let half = angle(&e.rig.pose, &e.rig.skeleton, "lip_corner.l");
    assert!(full > 0.05, "{full}");
    assert!((half - full * 0.5).abs() < 0.02, "half={half} full={full}");
}

#[test]
fn mirror_swaps_left_and_right() {
    let mut e = Editor::new(crate::samples::mannequin());
    let wink = face_preset("builtin.face.wink.l");
    e.apply_expression(&wink, 1.0, false, false, false).unwrap();
    let (l, r) = (
        angle(&e.rig.pose, &e.rig.skeleton, "eyelid_top.l"),
        angle(&e.rig.pose, &e.rig.skeleton, "eyelid_top.r"),
    );
    assert!(l > 0.3 && r < 0.05, "wink gauche : {l}/{r}");
    e.undo();
    e.apply_expression(&wink, 1.0, true, false, false).unwrap();
    let (l, r) = (
        angle(&e.rig.pose, &e.rig.skeleton, "eyelid_top.l"),
        angle(&e.rig.pose, &e.rig.skeleton, "eyelid_top.r"),
    );
    assert!(r > 0.3 && l < 0.05, "miroir : {l}/{r}");
}

#[test]
fn additive_layers_over_existing_face_pose() {
    let mut e = Editor::new(crate::samples::mannequin());
    e.apply_expression(&face_preset("builtin.face.happy"), 1.0, false, false, false)
        .unwrap();
    let corner = e.rig.skeleton.find("lip_corner.l").unwrap();
    let smiled = e.rig.pose.get(corner);
    // Clignement additif : les paupières bougent, le sourire reste.
    e.apply_expression(&face_preset("builtin.face.blink"), 1.0, false, true, false)
        .unwrap();
    assert!(angle(&e.rig.pose, &e.rig.skeleton, "eyelid_top.l") > 0.3);
    assert!(e.rig.pose.get(corner).abs_diff_eq(&smiled, 1e-5), "sourire perdu");
}

#[test]
fn apply_as_key_writes_a_key() {
    let mut e = Editor::new(crate::samples::mannequin());
    e.set_frame(5.0);
    e.apply_expression(&face_preset("builtin.face.surprise"), 1.0, false, false, true)
        .unwrap();
    assert!(e.rig.clip.key_at(5).is_some());
    // Sans clé, seule la pose change (pas de nouvelle clé).
    let mut e2 = Editor::new(crate::samples::mannequin());
    e2.set_frame(7.0);
    e2.apply_expression(&face_preset("builtin.face.surprise"), 1.0, false, false, false)
        .unwrap();
    assert!(e2.rig.clip.key_at(7).is_none());
}

#[test]
fn skips_missing_face_bones_gracefully() {
    let mut s = Skeleton::new();
    let head = s.add_bone_world(
        "head.x",
        None,
        Vec3::new(0.0, 1.46, 0.0),
        Vec3::new(0.0, 1.75, 0.0),
    );
    s.add_bone_world(
        "jaw.x",
        Some(head),
        Vec3::new(0.0, 1.61, 0.0),
        Vec3::new(0.0, 1.55, 0.1),
    );
    s.add_bone_world(
        "eye.l",
        Some(head),
        Vec3::new(0.05, 1.62, 0.08),
        Vec3::new(0.05, 1.62, 0.1),
    );
    s.add_bone_world(
        "eye.r",
        Some(head),
        Vec3::new(-0.05, 1.62, 0.08),
        Vec3::new(-0.05, 1.62, 0.1),
    );
    let mut rig = Rig::new("partial", Mesh::default());
    rig.set_skeleton(s, Vec::new());
    let mut e = Editor::new(rig);
    let report = e
        .apply_expression(&face_preset("builtin.face.surprise"), 1.0, false, false, false)
        .unwrap();
    assert!(report.applied.iter().any(|n| n == "jaw.x"));
    assert!(report.skipped.iter().any(|n| n.starts_with("brow")));
    let jaw = e.rig.skeleton.find("jaw.x").unwrap();
    assert!(e.rig.pose.get(jaw).rotation.is_finite());
}

#[test]
fn foreign_face_names_resolve() {
    // Rigify / Auto-Rig Pro.
    assert_eq!(foreign("eye.L"), Some(FaceBone::EyeL));
    assert_eq!(foreign("jaw_master"), Some(FaceBone::Jaw));
    assert_eq!(foreign("brow.T.L"), Some(FaceBone::BrowMidL));
    assert_eq!(foreign("lip_corner.R"), Some(FaceBone::LipCornerR));
    // Character Creator / iClone.
    assert_eq!(foreign("CC_Base_JawRoot"), Some(FaceBone::Jaw));
    assert_eq!(foreign("CC_Base_L_Eye"), Some(FaceBone::EyeL));
    assert_eq!(foreign("CC_Base_Teeth01"), Some(FaceBone::TeethTop));
    assert_eq!(foreign("CC_Base_Tongue01"), Some(FaceBone::Tongue1));
    // Noms anglais divers.
    assert_eq!(foreign("LeftUpperEyelid"), Some(FaceBone::EyelidTopL));
    // Os du corps : aucune correspondance.
    assert_eq!(foreign("upperarm_l"), None);
    assert_eq!(foreign("spine.003"), None);
}

#[test]
fn imported_rig_is_posed_through_mapping() {
    let mut s = Skeleton::new();
    let head = s.add_bone_world("Head", None, Vec3::new(0.0, 1.46, 0.0), Vec3::new(0.0, 1.75, 0.0));
    s.add_bone_world(
        "CC_Base_JawRoot",
        Some(head),
        Vec3::new(0.0, 1.61, 0.0),
        Vec3::new(0.0, 1.55, 0.1),
    );
    let mapped = super::present_mapped(&s);
    assert!(mapped.iter().any(|(b, _)| *b == FaceBone::Jaw));
    let mut rig = Rig::new("imported", Mesh::default());
    rig.set_skeleton(s, Vec::new());
    let mut e = Editor::new(rig);
    let report = e
        .apply_expression(&face_preset("builtin.face.jaw_drop"), 1.0, false, false, false)
        .unwrap();
    assert!(report.applied.iter().any(|n| n == "jaw.x"));
    let jaw = e.rig.skeleton.find("CC_Base_JawRoot").unwrap();
    assert!(e.rig.pose.get(jaw).rotation.to_axis_angle().1 > 0.1);
}

#[test]
fn expressions_are_listed_under_facial_expression() {
    let all = crate::presets::builtin();
    let faces: Vec<_> = all.iter().filter(is_expr).collect();
    assert!(faces.len() >= 30, "{}", faces.len());
    assert!(faces.iter().all(|p| p.info.license == "CC0-1.0"));
    assert!(faces.iter().all(|p| matches!(p.kind, PresetKind::Expression(_))));
    for g in ["Emotions", "Common", "Visemes", "Animals"] {
        assert!(faces.iter().any(|p| p.info.group == g), "groupe {g} absent");
    }
    let ids: std::collections::HashSet<_> = faces.iter().map(|p| &p.info.id).collect();
    assert_eq!(ids.len(), faces.len());
}

#[test]
fn preview_renders_an_expression() {
    let preview = crate::presets::Preview::new(&face_preset("builtin.face.happy")).unwrap();
    let mut seg = Vec::new();
    preview.segments(0.0, &mut seg);
    assert_eq!(seg.len(), preview.skeleton().len());
    assert!(seg.iter().all(|[a, b]| a.is_finite() && b.is_finite()));
    // La tête de référence a bien un visage (langue comprise pour les visèmes).
    assert!(preview.skeleton().find("tongue_01.x").is_some());
}
