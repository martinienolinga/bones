use glam::{Vec2, Vec3};

use super::{head_bounds, is_head_name};
use crate::autorig::face::FaceMarkerKind;
use crate::{Editor, Markers, Mesh, Skeleton, Viewport, samples};

/// Maillage déplacé et mis à l'échelle (modèle importé quelconque).
fn transformed(mesh: &Mesh, scale: f32, offset: Vec3) -> Mesh {
    let mut out = mesh.clone();
    for p in &mut out.positions {
        *p = *p * scale + offset;
    }
    out
}

/// Tête plausible : en haut du modèle, d'une hauteur d'environ un septième
/// à un cinquième de celle du corps, plus étroite que les épaules.
fn assert_head(mesh: &Mesh, (lo, hi): (Vec3, Vec3)) {
    let (mlo, mhi) = mesh.bounds().unwrap();
    let height = mhi.y - mlo.y;
    let size = hi - lo;
    assert!(
        (mhi.y - hi.y).abs() < 0.03 * height,
        "sommet du crâne : {hi} / {mhi}"
    );
    assert!((0.09..0.22).contains(&(size.y / height)), "hauteur : {size}");
    assert!(size.x < 0.2 * height, "largeur (sans les bras) : {size}");
}

#[test]
fn mannequin_head_is_found_from_every_hint() {
    let rig = samples::mannequin();
    let mesh = &rig.mesh;
    let markers = Markers::guess(mesh).unwrap();
    let face = crate::FaceMarkers::guess(mesh, &markers).expect("visage du mannequin");
    // 1. Marqueurs : tous les marqueurs du visage sont dans la boîte.
    let (lo, hi) = head_bounds(mesh, &rig.skeleton, Some(&markers), Some(&face)).unwrap();
    assert_head(mesh, (lo, hi));
    for p in FaceMarkerKind::ALL.iter().filter_map(|k| face.get(*k)) {
        assert!(
            p.cmpge(lo - 1e-5).all() && p.cmple(hi + 1e-5).all(),
            "{p} hors de la tête"
        );
    }
    // 2. Os de la tête seul, 3. maillage seul.
    let by_bone = head_bounds(mesh, &rig.skeleton, None, None).unwrap();
    assert_head(mesh, by_bone);
    let by_mesh = head_bounds(mesh, &Skeleton::new(), None, None).unwrap();
    assert_head(mesh, by_mesh);
}

#[test]
fn offset_and_scaled_models_keep_their_head() {
    let base = samples::mannequin_mesh();
    let reference = head_bounds(&base, &Skeleton::new(), Markers::guess(&base).as_ref(), None).unwrap();
    for (scale, offset) in [
        (100.0, Vec3::new(250.0, -40.0, 900.0)),
        (0.02, Vec3::new(-3.0, 7.0, 0.5)),
    ] {
        let mesh = transformed(&base, scale, offset);
        let markers = Markers::guess(&mesh);
        let (lo, hi) = head_bounds(&mesh, &Skeleton::new(), markers.as_ref(), None).unwrap();
        assert_head(&mesh, (lo, hi));
        let expected = (reference.0 * scale + offset, reference.1 * scale + offset);
        let tolerance = 1e-3 * scale;
        assert!(lo.abs_diff_eq(expected.0, tolerance) && hi.abs_diff_eq(expected.1, tolerance));
    }
}

#[test]
fn head_bone_names_are_recognized() {
    for name in ["Head", "mixamorig:Head", "head_jnt", "J_Bip_C_Head"] {
        assert!(is_head_name(name), "{name}");
    }
    for name in ["HeadTop_End", "head_end", "Neck", "head_nub"] {
        assert!(!is_head_name(name), "{name}");
    }
}

#[test]
fn empty_models_have_no_head() {
    assert!(head_bounds(&Mesh::default(), &Skeleton::new(), None, None).is_none());
}

#[test]
fn face_camera_frames_the_markers_from_the_front() {
    let mut editor = Editor::new(samples::mannequin());
    editor.guess_markers();
    let face = editor.face_markers.expect("visage du mannequin");
    let viewport = Viewport::new(2732.0, 2048.0);
    let safe = Viewport {
        x: 200.0,
        y: 180.0,
        width: 2332.0,
        height: 1500.0,
    };
    let camera = editor.face_camera(&viewport, &safe).expect("cadrage de la tête");
    assert_eq!((camera.yaw, camera.pitch), (0.0, 0.0), "vue de face");
    let (min, max) = (Vec2::new(safe.x, safe.y), Vec2::new(2532.0, 1680.0));
    let mut span = (f32::MAX, f32::MIN);
    for p in FaceMarkerKind::ALL.iter().filter_map(|k| face.get(*k)) {
        let s = camera.project(p, &viewport).unwrap();
        assert!(
            s.cmpge(min).all() && s.cmple(max).all(),
            "{s} hors de la zone utile"
        );
        span = (span.0.min(s.y), span.1.max(s.y));
    }
    // Gros plan : du sourcil au menton, le visage occupe une bonne part de la zone.
    assert!(
        span.1 - span.0 > 0.3 * safe.height,
        "visage trop petit : {span:?}"
    );
    // Le cadrage d'avant n'est pas touché (simple calcul).
    assert_ne!(editor.camera, camera);
    editor.camera = camera;
    assert!(!editor.can_undo(), "un cadrage ne crée pas d'étape d'annulation");
}
