//! Informations dessinées sur la vue 3D : statistiques (coin haut gauche,
//! comme Nomad Sculpt), étiquettes des marqueurs, message temporaire et
//! indicateur de calcul.

use bones_core::{Editor, Handle, Mode, Viewport};
use bones_ui::theme;

use super::{Busy, UiState};

const TOAST_SECONDS: f64 = 2.5;

pub fn show(ctx: &egui::Context, editor: &Editor, state: &mut UiState, viewport: &Viewport, busy: Busy) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let ppp = ctx.pixels_per_point();
    stats(&painter, editor);
    if editor.mode == Mode::AutoRig {
        marker_labels(&painter, editor, viewport, ppp);
    }
    toast(ctx, state);
    if busy.binding || busy.jobs > 0 {
        let text = if busy.binding {
            "Calcul des poids…"
        } else {
            "Traitement…"
        };
        egui::Area::new(egui::Id::unique("busy"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 64.0])
            .interactable(false)
            .show(ctx, |ui| {
                theme::bar_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(text);
                    });
                });
            });
    }
}

fn stats(painter: &egui::Painter, editor: &Editor) {
    let rig = &editor.rig;
    let mut lines = vec![
        format!(
            "{} sommets · {} triangles",
            rig.mesh.vertex_count(),
            rig.mesh.triangle_count()
        ),
        format!("{} os · {} clés", rig.skeleton.len(), rig.clip.keys().len()),
    ];
    if let Some(bone) = editor.selected {
        let b = rig.skeleton.bone(bone);
        lines.push(format!("Os : {}  ({:.3} m)", b.name, b.length));
    }
    let origin = egui::pos2(64.0, 70.0);
    let font = egui::FontId::proportional(12.0);
    for (i, line) in lines.iter().enumerate() {
        painter.text(
            origin + egui::vec2(0.0, i as f32 * 16.0),
            egui::Align2::LEFT_TOP,
            line,
            font.clone(),
            theme::TEXT_DIM,
        );
    }
}

fn marker_labels(painter: &egui::Painter, editor: &Editor, viewport: &Viewport, ppp: f32) {
    let font = egui::FontId::proportional(13.0);
    for (handle, p) in editor.handles() {
        let Handle::Marker(kind, mirrored) = handle else {
            continue;
        };
        if mirrored {
            continue;
        }
        if let Some(s) = editor.camera.project(p, viewport) {
            let pos = egui::pos2(s.x / ppp + 14.0, s.y / ppp - 8.0);
            painter.text(
                pos,
                egui::Align2::LEFT_TOP,
                kind.label(),
                font.clone(),
                theme::TEXT,
            );
        }
    }
}

fn toast(ctx: &egui::Context, state: &mut UiState) {
    let now = ctx.input(|i| i.time);
    let Some((message, shown)) = &mut state.toast else {
        return;
    };
    if shown.is_nan() {
        *shown = now;
    }
    if now - *shown > TOAST_SECONDS {
        state.toast = None;
        return;
    }
    let message = message.clone();
    egui::Area::new(egui::Id::unique("toast"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 110.0])
        .interactable(false)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            theme::panel_frame().show(ui, |ui| ui.label(message));
        });
    ctx.request_repaint_after(std::time::Duration::from_millis(200));
}
