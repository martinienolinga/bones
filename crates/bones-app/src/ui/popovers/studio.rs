//! Mise en page « studio », à la Brush Studio de Procreate : colonne de
//! navigation à gauche (titre, sections en pilules), et à droite le titre de
//! la section, les boutons de validation et les réglages qui défilent.

use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

/// Dessine le studio ; `buttons` remplit le coin haut droit (mise en page
/// de droite à gauche), `content` reçoit la section courante.
pub fn show(
    ui: &mut egui::Ui,
    title: &str,
    items: &[(Icon, &str)],
    section: &mut usize,
    height: f32,
    buttons: impl FnOnce(&mut egui::Ui),
    content: impl FnOnce(&mut egui::Ui, usize),
) {
    *section = (*section).min(items.len().saturating_sub(1));
    ui.horizontal_top(|ui| {
        egui::Frame::new()
            .fill(theme::NAV)
            .corner_radius(egui::CornerRadius::same(16))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(196.0);
                ui.set_height(height - 24.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(title).font(theme::semibold(26.0)));
                    });
                    ui.add_space(16.0);
                    ui.spacing_mut().item_spacing.y = 8.0;
                    for (i, (icon, label)) in items.iter().enumerate() {
                        if widgets::nav_item(ui, *icon, label, *section == i).clicked() {
                            *section = i;
                        }
                    }
                });
            });
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.set_height(height);
            ui.horizontal(|ui| {
                ui.set_min_height(44.0);
                widgets::studio_title(ui, items[*section].1);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), buttons);
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt((title, *section))
                .auto_shrink([false, false])
                .max_height(height - 60.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    content(ui, *section);
                    ui.add_space(8.0);
                });
        });
    });
}
