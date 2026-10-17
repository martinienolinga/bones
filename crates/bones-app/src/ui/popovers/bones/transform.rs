//! Section « Transform » : tête et queue (X / Y / Z, en mètres), longueur,
//! roulis (en degrés) et orientation (retourner, aligner sur un axe). Les
//! réglages continus d'une même valeur ne font qu'une étape d'annulation.

use bones_core::Editor;
use bones_core::editor::BoneInfo;
use bones_ui::{theme, widgets};
use glam::Vec3;

const AXES: [(&str, egui::Color32); 3] = [
    ("X", egui::Color32::from_rgb(0xff, 0x45, 0x3a)),
    ("Y", egui::Color32::from_rgb(0x30, 0xd1, 0x58)),
    ("Z", egui::Color32::from_rgb(0x0a, 0x84, 0xff)),
];

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, info: &BoneInfo) {
    widgets::section_title(ui, "Position (m)");
    let (mut head, mut tail) = (info.head, info.tail);
    let (head_changed, tail_changed) = widgets::group(ui, |g| {
        let h = g.custom(50.0, |ui| vec3_row(ui, "Head", &mut head));
        let t = g.custom(50.0, |ui| vec3_row(ui, "Tail", &mut tail));
        (h, t)
    });
    if head_changed {
        editor.set_selected_head(head);
    }
    if tail_changed {
        editor.set_selected_tail(tail);
    }

    widgets::section_title(ui, "Size & roll");
    let mut length = info.length;
    let chip = |ui: &mut egui::Ui, v: &mut f32| widgets::drag_chip(ui, v, 0.001..=10.0, 0.002, 3, " m", 96.0);
    if widgets::slider_row_with(ui, &mut length, 0.01..=2.0, "Length", chip).changed() {
        editor.set_selected_length(length.max(0.001));
    }
    let mut roll = info.roll.to_degrees();
    let chip = |ui: &mut egui::Ui, v: &mut f32| widgets::drag_chip(ui, v, -180.0..=180.0, 0.5, 0, "°", 80.0);
    if widgets::slider_row_with(ui, &mut roll, -180.0..=180.0, "Roll", chip).changed() {
        editor.set_selected_roll(roll.to_radians());
    }

    widgets::section_title(ui, "Orientation");
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.spacing_mut().item_spacing.x = 6.0;
        if widgets::chip_button(ui, "Flip", false)
            .on_hover_text("Swap head and tail")
            .clicked()
        {
            editor.flip_selected();
        }
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Align").size(15.0).color(theme::TEXT_DIM));
        for (axis, (name, _)) in [Vec3::X, Vec3::Y, Vec3::Z].into_iter().zip(AXES) {
            if widgets::chip_button(ui, name, false)
                .on_hover_text(format!("Point the bone along {name}"))
                .clicked()
            {
                editor.align_selected(axis);
            }
        }
    });
}

/// Libellé puis trois pastilles X / Y / Z ; vrai si une valeur a changé.
fn vec3_row(ui: &mut egui::Ui, label: &str, v: &mut Vec3) -> bool {
    ui.add_sized(
        [52.0, 30.0],
        egui::Label::new(egui::RichText::new(label).size(17.0).color(theme::TEXT_LIST)),
    );
    let width = ((ui.available_width() - 3.0 * 22.0) / 3.0 - 6.0).max(56.0);
    let mut changed = false;
    for (i, (name, color)) in AXES.iter().enumerate() {
        ui.label(
            egui::RichText::new(*name)
                .font(theme::semibold(13.0))
                .color(*color),
        );
        changed |= widgets::drag_chip(ui, &mut v[i], -100.0..=100.0, 0.002, 3, "", width).changed();
    }
    changed
}
