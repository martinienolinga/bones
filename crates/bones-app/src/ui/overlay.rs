//! Informations dessinées sur la vue 3D : statistiques (coin haut gauche,
//! comme Nomad Sculpt), étiquettes des marqueurs, message temporaire animé,
//! indicateur de calcul et consigne de l'outil « Parent ».

use bones_core::{Editor, Handle, Mode, Viewport};
use bones_ui::icons::{self, Icon};
use bones_ui::theme;

use super::{BAR_H, Busy, EDGE, UiState};

const TOAST_SECONDS: f64 = 2.6;
const FADE: f64 = 0.18;

/// Retourne la zone de la consigne, s'il y en a une (elle est touchable).
pub fn show(
    ctx: &egui::Context,
    editor: &Editor,
    state: &mut UiState,
    viewport: &Viewport,
    busy: Busy,
) -> Option<egui::Rect> {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let ppp = ctx.pixels_per_point();
    stats(&painter, editor);
    if editor.mode == Mode::AutoRig {
        marker_labels(&painter, editor, viewport, ppp);
    }
    toast(ctx, state);
    if busy.binding || busy.jobs > 0 {
        let text = if busy.binding {
            "Computing weights…"
        } else {
            "Working…"
        };
        egui::Area::new(egui::Id::unique("busy"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, BAR_H + 12.0])
            .interactable(false)
            .show(ctx, |ui| {
                theme::bar_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        ui.add(egui::Spinner::new().size(18.0).color(theme::ACCENT));
                        ui.label(egui::RichText::new(text).font(theme::semibold(14.0)));
                        ui.add_space(6.0);
                    });
                });
            });
    }
    hint(ctx, editor, state)
}

fn stats(painter: &egui::Painter, editor: &Editor) {
    let rig = &editor.rig;
    let mut lines = vec![
        format!(
            "{} vertices · {} triangles",
            rig.mesh.vertex_count(),
            rig.mesh.triangle_count()
        ),
        format!("{} bones · {} keys", rig.skeleton.len(), rig.clip.keys().len()),
    ];
    if let Some(bone) = editor.selected {
        let b = rig.skeleton.bone(bone);
        lines.push(format!("Bone: {}  ·  {:.3} m", b.name, b.length));
    }
    let origin = egui::pos2(16.0, BAR_H + 12.0);
    for (i, line) in lines.iter().enumerate() {
        let (font, color) = if i == 2 {
            (theme::semibold(13.0), theme::BONE_SELECTED)
        } else {
            (egui::FontId::proportional(12.0), theme::TEXT_DIM)
        };
        painter.text(
            origin + egui::vec2(0.0, i as f32 * 17.0),
            egui::Align2::LEFT_TOP,
            line,
            font,
            color,
        );
    }
}

fn marker_labels(painter: &egui::Painter, editor: &Editor, viewport: &Viewport, ppp: f32) {
    let font = theme::semibold(12.0);
    for (handle, p) in editor.handles() {
        let Handle::Marker(kind, mirrored) = handle else {
            continue;
        };
        if mirrored {
            continue;
        }
        if let Some(s) = editor.camera.project(p, viewport) {
            let galley = painter.layout_no_wrap(kind.label().to_owned(), font.clone(), theme::TEXT);
            let pos = egui::pos2(s.x / ppp + 14.0, s.y / ppp - galley.size().y * 0.5 - 3.0);
            let pill = egui::Rect::from_min_size(pos, galley.size() + egui::vec2(14.0, 6.0));
            painter.rect_filled(pill, 8, egui::Color32::from_black_alpha(150));
            painter.rect_stroke(
                pill,
                8,
                egui::Stroke::new(1.0, theme::MARKER.gamma_multiply(0.6)),
                egui::StrokeKind::Inside,
            );
            painter.galley(pill.min + egui::vec2(7.0, 3.0), galley, theme::TEXT);
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
    let age = now - *shown;
    if age > TOAST_SECONDS {
        state.toast = None;
        return;
    }
    // Apparition et disparition en fondu, avec un léger glissement.
    let alpha = (age / FADE).min((TOAST_SECONDS - age) / FADE).clamp(0.0, 1.0) as f32;
    let message = message.clone();
    egui::Area::new(egui::Id::unique("toast"))
        .anchor(
            egui::Align2::CENTER_TOP,
            [0.0, BAR_H + 60.0 - 10.0 * (1.0 - alpha)],
        )
        .interactable(false)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.set_opacity(alpha);
            theme::bar_frame()
                .inner_margin(egui::Margin::symmetric(16, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (dot, _) = ui.allocate_exact_size(egui::vec2(10.0, 18.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot.center(), 3.5, theme::ACCENT);
                        ui.label(egui::RichText::new(message).font(theme::semibold(14.0)));
                    });
                });
        });
    ctx.request_repaint();
}

/// Consigne de l'outil « Parent », en pastille au bas de l'écran.
fn hint(ctx: &egui::Context, editor: &Editor, state: &UiState) -> Option<egui::Rect> {
    let reparenting = editor.mode == Mode::Edit && editor.edit_tool == bones_core::EditTool::Reparent;
    if !reparenting {
        return None;
    }
    let text = state
        .hint
        .as_deref()
        .unwrap_or("Tap a bone, then its new parent (or empty space for none)");
    let area = egui::Area::new(egui::Id::unique("reparent-hint"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -EDGE - 14.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            theme::bar_frame()
                .corner_radius(22)
                .inner_margin(egui::Margin::symmetric(18, 12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (glyph, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                        icons::paint(ui.painter(), glyph, Icon::Hierarchy, theme::ACCENT);
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(text).size(15.0).color(theme::TEXT));
                    });
                });
        });
    Some(area.response.rect)
}
