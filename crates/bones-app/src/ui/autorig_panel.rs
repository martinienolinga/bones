//! Panneau du rigging assisté (mode Auto-Rig), dans l'esprit du mode
//! « Smart » d'Auto-Rig Pro : ajuster les marqueurs, choisir les options,
//! générer.

use bones_core::Editor;
use bones_ui::icons::{self, Icon};
use bones_ui::{theme, widgets};

use super::{EDGE, block};

pub fn show(ctx: &egui::Context, editor: &mut Editor) -> egui::Rect {
    block(
        ctx,
        "autorig",
        egui::Align2::CENTER_BOTTOM,
        [0.0, -EDGE],
        theme::panel_frame(),
        |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (tile, _) = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover());
                let tone = Icon::AutoRig.tone();
                ui.painter().rect_filled(
                    tile,
                    12,
                    egui::Color32::from_rgba_unmultiplied(tone.r(), tone.g(), tone.b(), 46),
                );
                icons::paint(
                    ui.painter(),
                    tile.shrink(9.0),
                    Icon::AutoRig,
                    egui::Color32::WHITE,
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(egui::RichText::new("Auto-rig").font(theme::semibold(17.0)));
                    ui.label(
                        egui::RichText::new(
                            "1. Place the markers (front view)  2. Set the options  3. Generate",
                        )
                        .size(13.0)
                        .color(theme::TEXT_DIM),
                    );
                });
                ui.add_space(12.0);
                let options = &mut editor.autorig_options;
                field(ui, "Spine", |ui| {
                    ui.add(egui::DragValue::new(&mut options.spine_count).range(1..=6))
                });
                field(ui, "Neck", |ui| {
                    ui.add(egui::DragValue::new(&mut options.neck_count).range(1..=3))
                });
                field(ui, "Fingers", |ui| widgets::switch(ui, &mut options.fingers));
                field(ui, "Toes", |ui| widgets::switch(ui, &mut options.toes));
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

/// Petit champ libellé au-dessus de son contrôle.
fn field(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> egui::Response) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.label(egui::RichText::new(label).size(12.0).color(theme::TEXT_DIM));
        add(ui);
    });
}
