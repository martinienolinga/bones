use glam::{Vec2, Vec3};

use super::corners;
use crate::{OrbitCamera, Viewport};

const VP: Viewport = Viewport {
    x: 0.0,
    y: 0.0,
    width: 2732.0,
    height: 2048.0,
};

/// Zone utile type : sous la barre du haut, au-dessus d'un panneau du bas.
const SAFE: Viewport = Viewport {
    x: 300.0,
    y: 200.0,
    width: 2132.0,
    height: 1500.0,
};

fn front() -> OrbitCamera {
    OrbitCamera {
        yaw: 0.0,
        pitch: 0.0,
        ..Default::default()
    }
}

/// Rectangle écran (min, max) des coins projetés.
fn screen_box(camera: &OrbitCamera, lo: Vec3, hi: Vec3) -> (Vec2, Vec2) {
    corners(lo, hi)
        .iter()
        .map(|p| camera.project(*p, &VP).expect("devant la caméra"))
        .fold((Vec2::MAX, Vec2::MIN), |(a, b), s| (a.min(s), b.max(s)))
}

/// La boîte tient dans la zone, la touche sur un axe et y est centrée.
fn assert_framed(camera: &OrbitCamera, lo: Vec3, hi: Vec3) {
    let (min, max) = screen_box(camera, lo, hi);
    let (smin, smax) = (
        Vec2::new(SAFE.x, SAFE.y),
        Vec2::new(SAFE.x + SAFE.width, SAFE.y + SAFE.height),
    );
    assert!(
        min.cmpge(smin - 0.5).all() && max.cmple(smax + 0.5).all(),
        "{min} {max}"
    );
    let tight_x = (min.x - smin.x).abs() < 1.0 && (max.x - smax.x).abs() < 1.0;
    let tight_y = (min.y - smin.y).abs() < 1.0 && (max.y - smax.y).abs() < 1.0;
    assert!(tight_x || tight_y, "au plus près : {min} {max}");
    let center = (min + max) * 0.5;
    let safe_center = (smin + smax) * 0.5;
    assert!(
        center.abs_diff_eq(safe_center, 0.03 * SAFE.height),
        "centrée : {center} {safe_center}"
    );
}

#[test]
fn head_box_fills_the_safe_area_from_the_front() {
    let (lo, hi) = (Vec3::new(-0.1, 1.5, -0.12), Vec3::new(0.1, 1.75, 0.1));
    let camera = front().framing_box(lo, hi, &VP, &SAFE).unwrap();
    assert_framed(&camera, lo, hi);
    // Vue de face conservée, distance raisonnable pour une tête de 25 cm.
    assert_eq!((camera.yaw, camera.pitch), (0.0, 0.0));
    assert!((0.3..1.5).contains(&camera.distance), "{}", camera.distance);
}

#[test]
fn offset_and_scaled_models_are_framed_too() {
    for (offset, scale) in [
        (Vec3::new(12.0, -3.0, 40.0), 1.0),
        (Vec3::new(-0.5, 0.2, 0.0), 0.01),
        (Vec3::ZERO, 180.0),
    ] {
        let lo = offset + Vec3::new(-0.1, 1.5, -0.12) * scale;
        let hi = offset + Vec3::new(0.1, 1.75, 0.1) * scale;
        let camera = front().framing_box(lo, hi, &VP, &SAFE).unwrap();
        assert_framed(&camera, lo, hi);
        let reference = front().framing_box(lo - offset, hi - offset, &VP, &SAFE).unwrap();
        // La distance suit l'échelle du modèle, pas sa position.
        assert!((camera.distance - reference.distance).abs() < 1e-3 * reference.distance);
    }
}

#[test]
fn wide_boxes_and_other_orientations_fit() {
    // Boîte large : c'est la largeur qui limite.
    let (lo, hi) = (Vec3::new(-2.0, 0.0, -0.2), Vec3::new(2.0, 0.5, 0.2));
    let camera = front().framing_box(lo, hi, &VP, &SAFE).unwrap();
    assert_framed(&camera, lo, hi);
    let three_quarter = OrbitCamera::default().framing_box(lo, hi, &VP, &SAFE).unwrap();
    assert_framed(&three_quarter, lo, hi);
}

#[test]
fn orthographic_framing_fits_exactly() {
    let (lo, hi) = (Vec3::new(-0.1, 1.5, -0.12), Vec3::new(0.1, 1.75, 0.1));
    let ortho = OrbitCamera {
        orthographic: true,
        ..front()
    };
    let camera = ortho.framing_box(lo, hi, &VP, &SAFE).unwrap();
    assert!(camera.orthographic);
    assert_framed(&camera, lo, hi);
}

#[test]
fn degenerate_inputs_are_refused() {
    let (lo, hi) = (Vec3::ZERO, Vec3::ONE);
    let empty = Viewport::new(0.0, 0.0);
    assert!(front().framing_box(lo, hi, &VP, &empty).is_none());
    assert!(front().framing_box(Vec3::NAN, hi, &VP, &SAFE).is_none());
    // Un point seul se cadre (distance minimale), sans valeur infinie.
    let point = front().framing_box(lo, lo, &VP, &SAFE);
    assert!(point.is_none_or(|c| c.distance.is_finite()));
}

#[test]
fn blend_takes_the_short_way_and_zooms_evenly() {
    let a = OrbitCamera {
        yaw: 3.0,
        distance: 1.0,
        ..Default::default()
    };
    let b = OrbitCamera {
        yaw: -3.0,
        distance: 4.0,
        target: Vec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    };
    assert!(a.blend(&b, 0.0).same_view(&a));
    let end = a.blend(&b, 1.0);
    assert!(end.target.abs_diff_eq(b.target, 1e-6) && (end.distance - 4.0).abs() < 1e-5);
    // Même direction de vue (lacet modulo 2π) : on passe par π, pas par 0.
    assert!(end.eye().abs_diff_eq(b.eye(), 1e-4));
    let mid = a.blend(&b, 0.5);
    assert!((mid.distance - 2.0).abs() < 1e-5, "zoom géométrique");
    assert!(mid.yaw.cos() < -0.9, "par le chemin court : {}", mid.yaw);
}
