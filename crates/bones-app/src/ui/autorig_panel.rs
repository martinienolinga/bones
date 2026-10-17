//! Panneau du rigging assisté (mode Auto-Rig), dans l'esprit du mode
//! « Smart » d'Auto-Rig Pro : deux étapes (corps puis visage), ajuster les
//! marqueurs, choisir les options, générer.

use bones_core::Editor;
use bones_core::autorig::face;
use bones_ui::icons::{self, Icon};
use bones_ui::{theme, widgets};

use super::{EDGE, UiState, block};

/// Étape de l'auto-rig : marqueurs du corps, puis du visage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Step {
    #[default]
    Body,
    Face,
}

pub fn show(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) -> egui::Rect {
    // À l'entrée dans l'étape « Face », on estime les marqueurs du visage s'ils
    // n'existent pas encore (à partir des marqueurs du corps).
    if state.autorig_step == Step::Face
        && editor.face_markers.is_none()
        && let Some(markers) = editor.markers
    {
        editor.face_markers = face::FaceMarkers::guess(&editor.rig.mesh, &markers);
    }
    block(
        ctx,
        "autorig",
        egui::Align2::CENTER_BOTTOM,
        [0.0, -EDGE],
        theme::panel_frame(),
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                heading(ui, state);
                ui.add_space(8.0);
                match state.autorig_step {
                    Step::Body => body(ui, editor),
                    Step::Face => face_step(ui, editor),
                }
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    if widgets::pill_button(ui, Some(Icon::AutoRig), "Generate rig", true).clicked() {
                        editor.generate_rig();
                    }
                });
            });
        },
    )
}

/// Glyphe, titre, bascule d'étape et consigne.
fn heading(ui: &mut egui::Ui, state: &mut UiState) {
    let (glyph, _) = ui.allocate_exact_size(egui::vec2(36.0, 48.0), egui::Sense::hover());
    icons::paint(
        ui.painter(),
        egui::Rect::from_center_size(glyph.center(), egui::vec2(28.0, 28.0)),
        Icon::AutoRig,
        theme::ACCENT,
    );
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Auto-rig").font(theme::semibold(20.0)));
            ui.add_space(6.0);
            let mut step = if state.autorig_step == Step::Face { 1 } else { 0 };
            if widgets::segmented(ui, &["Body", "Face"], &mut step).changed() {
                state.autorig_step = if step == 1 { Step::Face } else { Step::Body };
            }
        });
        let hint = match state.autorig_step {
            Step::Body => "1. Place the markers (front view)   2. Set the options   3. Generate",
            Step::Face => "Drag the face markers onto the head, then generate with Face on.",
        };
        ui.label(egui::RichText::new(hint).size(13.0).color(theme::TEXT_DIM));
    });
}

/// Options du corps : colonne, cou, doigts, orteils.
fn body(ui: &mut egui::Ui, editor: &mut Editor) {
    let options = &mut editor.autorig_options;
    field(ui, "Spine", |ui| {
        ui.add(egui::DragValue::new(&mut options.spine_count).range(1..=6))
    });
    field(ui, "Neck", |ui| {
        ui.add(egui::DragValue::new(&mut options.neck_count).range(1..=3))
    });
    field(ui, "Fingers", |ui| widgets::switch(ui, &mut options.fingers));
    field(ui, "Toes", |ui| widgets::switch(ui, &mut options.toes));
}

/// Options du visage : bascule du rig facial et état de détection.
fn face_step(ui: &mut egui::Ui, editor: &mut Editor) {
    field(ui, "Face rig", |ui| {
        widgets::switch(ui, &mut editor.autorig_options.face)
    });
    let detected = editor.face_markers.is_some();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(egui::RichText::new("Markers").size(12.0).color(theme::TEXT_DIM));
        let (text, color) = if detected {
            ("Face detected", theme::SUCCESS)
        } else {
            ("No face found", theme::TEXT_DIM)
        };
        ui.label(egui::RichText::new(text).size(14.0).color(color));
    });
    ui.add_space(4.0);
    if widgets::tool_button(ui, Icon::Reset, false, "Re-estimate the face markers").clicked()
        && let Some(markers) = editor.markers
    {
        editor.face_markers = face::FaceMarkers::guess(&editor.rig.mesh, &markers);
    }
}

/// Petit champ libellé au-dessus de son contrôle.
fn field(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> egui::Response) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(egui::RichText::new(label).size(12.0).color(theme::TEXT_DIM));
        add(ui);
    });
}
