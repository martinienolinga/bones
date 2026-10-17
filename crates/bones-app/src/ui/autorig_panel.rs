//! Panneau du rigging assisté (mode Auto-Rig), dans l'esprit du mode
//! « Smart » d'Auto-Rig Pro : ajuster les marqueurs, choisir les options,
//! générer.

use bones_core::Editor;
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::block;

pub fn show(ctx: &egui::Context, editor: &mut Editor) -> egui::Rect {
    block(
        ctx,
        "autorig",
        egui::Align2::CENTER_BOTTOM,
        [0.0, -8.0],
        theme::panel_frame(),
        |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Rigging assisté").strong().size(17.0));
                    ui.label(
                        egui::RichText::new(
                            "1. Placez les marqueurs (vue de face)  2. Réglez les options  3. Générez",
                        )
                        .color(theme::TEXT_DIM),
                    );
                });
                ui.add_space(16.0);
                let options = &mut editor.autorig_options;
                ui.label("Colonne");
                ui.add(egui::DragValue::new(&mut options.spine_count).range(1..=6));
                ui.label("Cou");
                ui.add(egui::DragValue::new(&mut options.neck_count).range(1..=3));
                ui.checkbox(&mut options.fingers, "Doigts");
                ui.checkbox(&mut options.toes, "Orteils");
                ui.add_space(12.0);
                if widgets::tool_button(ui, Icon::AutoRig, true, "Générer le rig").clicked() {
                    editor.generate_rig();
                }
            });
        },
    )
}
