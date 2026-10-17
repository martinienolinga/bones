//! Overlay des marqueurs du visage (étape « Face » de l'auto-rig), sur le même
//! principe que les marqueurs du corps : des points manipulables posés sur la
//! tête du modèle. On ne règle que le côté gauche ; le côté droit (en pointillé
//! pâle) suit par symétrie, comme à la génération du rig.
//!
//! La manipulation se fait au niveau de l'interface (projection de la caméra),
//! sans toucher à la logique du cœur : chaque point glisse dans le plan
//! parallèle à l'écran passant par sa position.

use bones_core::autorig::face::{FaceMarkerKind, FaceMarkers};
use bones_core::{Editor, Viewport};
use bones_ui::theme;
use egui::{Color32, Sense, Stroke, vec2};
use glam::{Vec2, Vec3};

use super::BAR_H;

/// Dessine et manipule les marqueurs ; retourne les zones touchables (pour que
/// le geste de caméra ne démarre pas dessus).
pub fn show(ctx: &egui::Context, editor: &mut Editor, viewport: &Viewport) -> Vec<egui::Rect> {
    let Some(start) = editor.face_markers else {
        return Vec::new();
    };
    let ppp = ctx.pixels_per_point();
    let axis = start.nose_tip.x; // axe de symétrie du visage (os central)
    let mut current = start;
    let mut rects = Vec::new();

    for (i, kind) in FaceMarkerKind::ALL.into_iter().enumerate() {
        let Some(world) = start.get(kind) else { continue };
        let Some(screen) = editor.camera.project(world, viewport) else {
            continue;
        };
        let center = egui::pos2(screen.x / ppp, screen.y / ppp);
        if center.y < BAR_H + 6.0 {
            continue;
        }
        let (rect, moved) = dot(ctx, &editor.camera, viewport, i, world, center, ppp);
        rects.push(rect);
        if let Some(p) = moved {
            current.set(kind, p);
        }
    }
    labels(ctx, editor, viewport, &start, axis, ppp);
    if current != start {
        editor.face_markers = Some(current);
    }
    rects
}

/// Un point manipulable ; retourne sa zone et sa nouvelle position monde s'il
/// a bougé.
fn dot(
    ctx: &egui::Context,
    camera: &bones_core::OrbitCamera,
    viewport: &Viewport,
    index: usize,
    world: Vec3,
    center: egui::Pos2,
    ppp: f32,
) -> (egui::Rect, Option<Vec3>) {
    let area = egui::Area::new(egui::Id::unique(("face-marker", index)))
        .fixed_pos(center - vec2(22.0, 22.0))
        .order(egui::Order::Foreground)
        .movable(false)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click_and_drag());
            let active = response.is_pointer_button_down_on() || response.dragged();
            let r = if active { 8.0 } else { 7.0 };
            let fill = if active { theme::ACCENT } else { theme::MARKER };
            let painter = ui.painter();
            painter.circle_filled(rect.center(), r + 2.0, Color32::from_black_alpha(130));
            painter.circle_filled(rect.center(), r, fill);
            painter.circle_stroke(rect.center(), r, Stroke::new(1.5, Color32::WHITE));
            let mut moved = None;
            if response.dragged()
                && let Some(p) = response.interact_pointer_pos()
            {
                let phys = Vec2::new(p.x * ppp, p.y * ppp);
                moved = Some(camera.point_on_view_plane(phys, viewport, world));
            }
            moved
        });
    (area.response.rect, area.inner)
}

/// Étiquettes des marqueurs et point pâle du côté droit (symétrique).
fn labels(
    ctx: &egui::Context,
    editor: &Editor,
    viewport: &Viewport,
    markers: &FaceMarkers,
    axis: f32,
    ppp: f32,
) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("face-marker-labels"),
    ));
    let font = theme::semibold(11.0);
    for kind in FaceMarkerKind::ALL {
        let Some(world) = markers.get(kind) else { continue };
        if let Some(s) = editor.camera.project(world, viewport) {
            let galley = painter.layout_no_wrap(kind.label().to_owned(), font.clone(), theme::TEXT);
            let pos = egui::pos2(s.x / ppp + 13.0, s.y / ppp - galley.size().y * 0.5 - 2.0);
            let pill = egui::Rect::from_min_size(pos, galley.size() + vec2(12.0, 5.0));
            if pill.top() < BAR_H {
                continue;
            }
            painter.rect_filled(pill, 7, Color32::from_black_alpha(150));
            painter.galley(pill.min + vec2(6.0, 2.5), galley, theme::TEXT);
        }
        // Point pâle du symétrique droit (non manipulable).
        if !kind.is_central() {
            let mirror = Vec3::new(2.0 * axis - world.x, world.y, world.z);
            if let Some(s) = editor.camera.project(mirror, viewport) {
                let p = egui::pos2(s.x / ppp, s.y / ppp);
                if p.y > BAR_H {
                    painter.circle_filled(p, 5.0, theme::MARKER.gamma_multiply(0.4));
                    painter.circle_stroke(p, 5.0, Stroke::new(1.0, Color32::from_white_alpha(90)));
                }
            }
        }
    }
}
