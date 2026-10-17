//! Actions : liaison des poids, animation, export (comme le menu
//! « Partager » de Procreate, un format par ligne).

use bones_core::io::Format;
use bones_core::{BindMethod, Editor};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::UiAction;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, actions: &mut Vec<UiAction>) {
    widgets::section_title(ui, "Poids automatiques");
    let methods = [BindMethod::Heat, BindMethod::Envelope];
    let mut index = methods.iter().position(|m| *m == editor.bind_method).unwrap_or(0);
    if widgets::segmented(ui, &["Diffusion de chaleur", "Enveloppes"], &mut index).changed() {
        editor.bind_method = methods[index];
        editor.request_rebind();
    }
    ui.label(
        egui::RichText::new(match editor.bind_method {
            BindMethod::Heat => "Qualité (Baran & Popović 2007), calculé en tâche de fond.",
            BindMethod::Envelope => "Instantané, selon la distance aux os.",
        })
        .size(12.0)
        .color(theme::TEXT_DIM),
    );
    if widgets::pill_button(ui, Some(Icon::Weights), "Recalculer les poids", false).clicked() {
        editor.request_rebind();
    }

    ui.add_space(8.0);
    widgets::section_title(ui, "Animation");
    ui.horizontal(|ui| {
        ui.label("Images par seconde");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut fps = editor.rig.clip.fps;
            if ui
                .add_sized([72.0, 32.0], egui::DragValue::new(&mut fps).range(1.0..=120.0))
                .changed()
            {
                editor.rig.clip_mut().fps = fps;
            }
        });
    });

    ui.add_space(8.0);
    export(ui, actions);
}

/// Un format par ligne, avec ce qu'il emporte.
fn export(ui: &mut egui::Ui, actions: &mut Vec<UiAction>) {
    widgets::section_title(ui, "Exporter");
    ui.spacing_mut().item_spacing.y = 4.0;
    for format in Format::exportable().filter(|f| *f != Format::Bones) {
        let mut contents = vec!["maillage"];
        if format.supports_textures() {
            contents.push("textures");
        }
        if format.exports_skeleton() {
            contents.push("squelette");
        }
        if format.exports_animation() {
            contents.push("animation");
        }
        // « glTF binaire (Blender, Unity…) » : titre court, détails dessous.
        let label = format.label();
        let (title, detail) = label.split_once(" (").unwrap_or((label, ""));
        let mut subtitle = contents.join(" · ");
        if !detail.is_empty() {
            subtitle = format!("{} — {subtitle}", detail.trim_end_matches(')'));
        }
        let badge = format!(".{}", format.extension());
        let row =
            widgets::list_row(ui, Icon::Export, title, Some(&subtitle), Some(&badge)).on_hover_text(label);
        if row.clicked() {
            actions.push(UiAction::Export(format));
        }
    }
    ui.label(
        egui::RichText::new("Les formats sans squelette (OBJ, STL, PLY) exportent la pose courante.")
            .size(12.0)
            .color(theme::TEXT_DIM),
    );
}
