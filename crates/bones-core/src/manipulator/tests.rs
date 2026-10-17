use std::f32::consts::FRAC_PI_2;

use glam::{Quat, Vec2, Vec3};

use super::*;
use crate::math::camera::PITCH_LIMIT;
use crate::{OrbitCamera, Viewport};

const R: f32 = 96.0;

fn front_camera() -> OrbitCamera {
    OrbitCamera {
        yaw: 0.0,
        pitch: 0.0,
        ..OrbitCamera::default()
    }
}

fn viewport() -> Viewport {
    Viewport::new(1000.0, 800.0)
}

fn frame(camera: &OrbitCamera) -> Frame {
    Frame {
        origin: camera.target,
        orientation: Quat::IDENTITY,
    }
}

fn geometry(camera: &OrbitCamera, mode: ManipulatorMode) -> Geometry {
    let mut g = Geometry::default();
    let options = Options {
        mode,
        radius_px: R,
        axis_scale: true,
    };
    assert!(g.build(&Projector::new(camera, &viewport()), &frame(camera), &options));
    g
}

fn begin(camera: &OrbitCamera, part: Part, pos: Vec2) -> ManipDrag {
    ManipDrag::begin(part, frame(camera), Projector::new(camera, &viewport()), pos, R)
}

fn same_rotation(a: Quat, b: Quat) -> bool {
    a.abs_diff_eq(b, 1e-3) || a.abs_diff_eq(-b, 1e-3)
}

#[test]
fn x_arrow_follows_the_pointer() {
    let cam = front_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let start = c + Vec2::new(0.6 * R, 0.0);
    let mut drag = begin(&cam, Part::Axis(Axis::X), start);
    let delta = drag.update(start + Vec2::new(100.0, 0.0), None);
    let expected = 100.0 * Projector::new(&cam, &viewport()).world_per_px(cam.target);
    assert!(
        delta.translate.abs_diff_eq(Vec3::X * expected, 1e-4),
        "{delta:?} {expected}"
    );
    // Un mouvement perpendiculaire à l'axe ne change presque rien (la
    // perspective décale à peine le point le plus proche).
    let delta = drag.update(start + Vec2::new(100.0, 70.0), None);
    assert!((delta.translate.x - expected).abs() < expected * 0.02 && delta.translate.y == 0.0);
}

#[test]
fn distance_snapping_uses_steps() {
    let cam = front_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let mut drag = begin(&cam, Part::Axis(Axis::Y), c + Vec2::new(0.0, -50.0));
    let snap = Snap {
        angle: 15f32.to_radians(),
        distance: 0.05,
        scale: 0.1,
    };
    let delta = drag.update(c + Vec2::new(0.0, -137.0), Some(snap));
    let steps = delta.translate.y / 0.05;
    assert!(
        delta.translate.y > 0.0 && (steps - steps.round()).abs() < 1e-3,
        "{delta:?}"
    );
}

/// Vue de dessus : l'anneau Y fait face à l'observateur, la droite de
/// l'écran est +X et le haut −Z.
fn top_camera() -> OrbitCamera {
    OrbitCamera {
        yaw: 0.0,
        pitch: PITCH_LIMIT,
        ..OrbitCamera::default()
    }
}

#[test]
fn quarter_turn_on_y_ring_gives_ninety_degrees() {
    let cam = top_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let mut drag = begin(&cam, Part::Ring(Axis::Y), c + Vec2::new(R, 0.0));
    // Droite → haut de l'écran : sens trigonométrique vu de dessus.
    drag.update(c + Vec2::new(R, -R), None);
    let delta = drag.update(c + Vec2::new(0.0, -R), None);
    assert!(
        same_rotation(delta.rotate, Quat::from_rotation_y(FRAC_PI_2)),
        "{delta:?}"
    );
    // L'angle se déroule au-delà d'un demi-tour.
    for p in [
        Vec2::new(-R, -R),
        Vec2::new(-R, 0.0),
        Vec2::new(-R, R),
        Vec2::new(0.0, R),
    ] {
        drag.update(c + p, None);
    }
    assert!(
        (drag.value.to_degrees() - 270.0).abs() < 0.5,
        "{}",
        drag.value.to_degrees()
    );
}

#[test]
fn angle_snapping_rounds_to_step() {
    let cam = top_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let mut drag = begin(&cam, Part::Ring(Axis::Y), c + Vec2::new(R, 0.0));
    let snap = Snap {
        angle: 15f32.to_radians(),
        distance: 0.05,
        scale: 0.1,
    };
    let a = 50f32.to_radians();
    let delta = drag.update(c + Vec2::new(a.cos(), -a.sin()) * R, Some(snap));
    assert!(same_rotation(
        delta.rotate,
        Quat::from_rotation_y(45f32.to_radians())
    ));
}

#[test]
fn edge_on_ring_turns_with_linear_drag() {
    let cam = front_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    // L'anneau X est vu par la tranche (trait vertical) : le glisser vers le
    // bas fait avancer le point avant vers le bas, rotation positive.
    let mut drag = begin(&cam, Part::Ring(Axis::X), c + Vec2::new(0.0, -0.5 * R));
    let delta = drag.update(c + Vec2::new(0.0, -0.5 * R + R * 0.5), None);
    assert!(
        same_rotation(delta.rotate, Quat::from_rotation_x(0.5)),
        "{delta:?}"
    );
}

#[test]
fn view_ring_rotates_about_view_axis() {
    let cam = front_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let mut drag = begin(&cam, Part::ViewRing, c + Vec2::new(1.2 * R, 0.0));
    let delta = drag.update(c + Vec2::new(0.0, -1.2 * R), None);
    // Vue de face, l'axe de vue est +Z : sens trigonométrique = rotation +Z.
    assert!(
        same_rotation(delta.rotate, Quat::from_rotation_z(FRAC_PI_2)),
        "{delta:?}"
    );
}

#[test]
fn scale_handles_use_distance_ratio() {
    let cam = front_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let mut drag = begin(&cam, Part::ScaleAxis(Axis::X), c + Vec2::new(0.5 * R, 0.0));
    let delta = drag.update(c + Vec2::new(R, 3.0), None);
    assert!(
        delta.scale.abs_diff_eq(Vec3::new(2.0, 1.0, 1.0), 1e-3),
        "{delta:?}"
    );
    let mut drag = begin(&cam, Part::ScaleUniform, c + Vec2::new(0.0, 1.12 * R));
    let delta = drag.update(c + Vec2::new(0.0, 0.56 * R), None);
    assert!(delta.scale.abs_diff_eq(Vec3::splat(0.5), 1e-3), "{delta:?}");
}

#[test]
fn picking_prefers_handles_then_rings_then_trackball() {
    let cam = front_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let g = geometry(&cam, ManipulatorMode::Combined);
    assert_eq!(
        g.pick(c + Vec2::new(0.9 * R, 0.0), 12.0),
        Some(Part::Axis(Axis::X))
    );
    assert_eq!(
        g.pick(c + Vec2::new(0.0, -0.9 * R), 12.0),
        Some(Part::Axis(Axis::Y))
    );
    assert_eq!(g.pick(c, 12.0), Some(Part::Center));
    assert_eq!(g.pick(c + Vec2::new(-1.12 * R, 0.0), 12.0), Some(Part::ViewRing));
    assert_eq!(g.pick(c + Vec2::new(-3.0 * R, 0.0), 12.0), None);
    // L'axe Z pointe vers l'observateur : pas de flèche.
    assert!(g.elements().iter().all(|e| e.part != Some(Part::Axis(Axis::Z))));

    let g = geometry(&cam, ManipulatorMode::Rotate);
    assert_eq!(g.pick(c + Vec2::new(0.0, R), 10.0), Some(Part::Ring(Axis::Z)));
    assert_eq!(
        g.pick(c + Vec2::new(0.5 * R, 0.5 * R), 10.0),
        Some(Part::Trackball)
    );
    assert_eq!(g.pick(c + Vec2::new(0.0, 1.2 * R), 10.0), Some(Part::ViewRing));
}

#[test]
fn ring_halves_split_front_and_back() {
    let cam = OrbitCamera {
        yaw: 0.4,
        pitch: 0.5,
        ..OrbitCamera::default()
    };
    let g = geometry(&cam, ManipulatorMode::Rotate);
    let rings = |back: bool| {
        g.elements()
            .iter()
            .filter(|e| matches!(e.part, Some(Part::Ring(_))) && e.kind == Kind::Line && e.back == back)
            .count()
    };
    assert!(rings(true) >= 3 && rings(false) >= 3);
    // Les moitiés arrière sont dessinées avant les moitiés avant.
    let first_front = g
        .elements()
        .iter()
        .position(|e| matches!(e.part, Some(Part::Ring(_))) && !e.back);
    let last_back = g
        .elements()
        .iter()
        .rposition(|e| matches!(e.part, Some(Part::Ring(_))) && e.back);
    assert!(last_back < first_front);
}

#[test]
fn trackball_turns_like_the_pointer() {
    let cam = front_camera();
    let c = cam.project(cam.target, &viewport()).unwrap();
    let mut drag = begin(&cam, Part::Trackball, c);
    let delta = drag.update(c + Vec2::new(R * 0.5, 0.0), None);
    // Glisser vers la droite : le point avant (+Z) part vers la droite (+X).
    let front = delta.rotate * Vec3::Z;
    assert!(front.x > 0.4 && front.y.abs() < 1e-4, "{front:?}");
}
