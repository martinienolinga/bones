//! Alignements de vue animés (orientation de la caméra seulement).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use bones_core::Editor;
use bones_core::math::camera::PITCH_LIMIT;
use glam::Vec3;

pub const PITCH: f32 = PITCH_LIMIT;
/// Vue d'accueil : trois quarts face, légèrement plongeante.
pub const HOME: (f32, f32) = (0.35, 0.2);
const DURATION: f64 = 0.28;

/// Transition en cours, gardée dans la mémoire temporaire d'egui.
#[derive(Clone, Copy, Debug)]
pub struct ViewAnimation {
    from: (f32, f32),
    to: (f32, f32),
    start: f64,
}

fn id() -> egui::Id {
    egui::Id::unique("view-animation")
}

/// Angles (lacet, tangage) qui placent l'œil sur l'axe `direction` ; le
/// lacet courant est gardé pour les vues de dessus et de dessous.
pub fn angles_for(direction: Vec3, current_yaw: f32) -> (f32, f32) {
    match direction.to_array().map(|c| c.round() as i32) {
        [1, 0, 0] => (FRAC_PI_2, 0.0),
        [-1, 0, 0] => (-FRAC_PI_2, 0.0),
        [0, 0, -1] => (PI, 0.0),
        [0, 1, 0] => (current_yaw, PITCH),
        [0, -1, 0] => (current_yaw, -PITCH),
        _ => (0.0, 0.0),
    }
}

/// Démarre une transition vers `(lacet, tangage)` par le chemin le plus court.
pub fn animate_to(ctx: &egui::Context, editor: &Editor, (yaw, pitch): (f32, f32)) {
    let from = (editor.camera.yaw, editor.camera.pitch);
    let turn = (yaw - from.0 + PI).rem_euclid(TAU) - PI;
    let animation = ViewAnimation {
        from,
        to: (from.0 + turn, pitch),
        start: ctx.input(|i| i.time),
    };
    ctx.data_mut(|d| d.insert_temp(id(), animation));
    ctx.request_repaint();
}

pub fn cancel(ctx: &egui::Context) {
    ctx.data_mut(|d| d.remove::<ViewAnimation>(id()));
}

/// Fait avancer la transition (à appeler à chaque image).
pub fn step(ctx: &egui::Context, editor: &mut Editor) {
    let Some(animation) = ctx.data(|d| d.get_temp::<ViewAnimation>(id())) else {
        return;
    };
    let t = ((ctx.input(|i| i.time) - animation.start) / DURATION).clamp(0.0, 1.0) as f32;
    // Départ et arrivée en douceur.
    let ease = t * t * (3.0 - 2.0 * t);
    editor.camera.yaw = animation.from.0 + (animation.to.0 - animation.from.0) * ease;
    editor.camera.pitch = animation.from.1 + (animation.to.1 - animation.from.1) * ease;
    if t >= 1.0 {
        cancel(ctx);
    } else {
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bones_core::OrbitCamera;

    #[test]
    fn snapped_eye_lies_on_the_axis() {
        for axis in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z] {
            let (yaw, pitch) = angles_for(axis, 0.3);
            let camera = OrbitCamera {
                target: Vec3::ZERO,
                yaw,
                pitch,
                ..Default::default()
            };
            assert!(camera.eye().normalize().abs_diff_eq(axis, 1e-5), "{axis:?}");
        }
        let (yaw, pitch) = angles_for(Vec3::Y, 0.3);
        assert_eq!((yaw, pitch), (0.3, PITCH));
    }
}
