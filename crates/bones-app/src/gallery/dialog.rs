//! Confirmation de suppression, comme une alerte d'iOS : écran assombri,
//! panneau centré, « Cancel » et « Delete » en rouge (action destructrice).

use bones_ui::theme;
use egui::{Align2, Color32, CornerRadius, FontId, Sense, vec2};

use super::popover;
use super::{Gallery, GalleryAction};

const WIDTH: f32 = 340.0;

pub fn show(ctx: &egui::Context, gallery: &mut Gallery, actions: &mut Vec<GalleryAction>) {
    let id = egui::Id::unique("gallery-delete");
    let Some(paths) = gallery.confirm.clone() else {
        ctx.animate_bool_with_time(id, false, 0.0);
        return;
    };
    let t = ctx.animate_bool_with_time(id, true, 0.16);
    if popover::scrim(ctx, "gallery-delete-scrim", 0.45 * t) {
        gallery.confirm = None;
        return;
    }
    let (title, message) = match paths.as_slice() {
        [one] => {
            let name = gallery
                .library
                .get(one)
                .map_or_else(|| crate::files::stem(one), |c| c.name.clone());
            (format!("Delete “{name}”?"), "This can't be undone.")
        }
        many => (
            format!("Delete {} projects?", many.len()),
            "This can't be undone.",
        ),
    };
    let screen = ctx.content_rect();
    let mut choice = None;
    egui::Area::new(id.with("panel"))
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.set_opacity(t);
            theme::panel_frame()
                .inner_margin(egui::Margin::same(22))
                .show(ui, |ui| {
                    ui.set_width((WIDTH - 44.0).min(screen.width() - 80.0));
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(title)
                                .font(theme::semibold(19.0))
                                .color(theme::TEXT),
                        );
                        ui.add_space(2.0);
                        ui.label(egui::RichText::new(message).size(15.0).color(theme::TEXT_DIM));
                    });
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        let w = (ui.available_width() - 10.0) * 0.5;
                        if button(ui, "Cancel", w, false).clicked() {
                            choice = Some(false);
                        }
                        if button(ui, "Delete", w, true).clicked() {
                            choice = Some(true);
                        }
                    });
                });
        });
    match choice {
        Some(true) => {
            gallery.confirm = None;
            if let Some(selection) = &mut gallery.selection {
                selection.retain(|p| !paths.contains(p));
            }
            gallery.library.remove(&paths);
            actions.push(GalleryAction::Delete(paths));
        }
        Some(false) => gallery.confirm = None,
        None => {}
    }
}

/// Bouton pleine largeur de 46 pt ; `destructive` : aplat rouge.
fn button(ui: &mut egui::Ui, label: &str, width: f32, destructive: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 46.0), Sense::click());
    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.1,
    );
    let (fill, font) = if destructive {
        (theme::DANGER, theme::semibold(17.0))
    } else {
        (theme::WIDGET, FontId::proportional(17.0))
    };
    let fill = theme::mix(fill, Color32::BLACK, 0.2 * press);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(23), fill);
    painter.text(rect.center(), Align2::CENTER_CENTER, label, font, Color32::WHITE);
    response
}
