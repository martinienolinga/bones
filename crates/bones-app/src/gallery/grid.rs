//! Grille défilante des cartes (seules les cartes visibles sont
//! construites) et état vide.

use bones_ui::icons::{self, Icon};
use bones_ui::theme;

use super::layout::{BAR_H, GRID_TOP, Grid};
use super::{Gallery, GalleryAction, card};

pub fn show(ui: &mut egui::Ui, gallery: &mut Gallery, actions: &mut Vec<GalleryAction>) {
    let screen = ui.ctx().content_rect();
    let area = egui::Rect::from_min_max(egui::pos2(screen.left(), screen.top() + BAR_H), screen.max);
    let grid = Grid::new(screen.width());
    gallery.shown.clear();

    if gallery.library.is_listed() && gallery.library.cards().is_empty() {
        empty_state(ui, area);
        return;
    }

    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(area));
    let count = gallery.library.cards().len();
    let mut scroll = egui::ScrollArea::vertical()
        .id_salt("gallery-grid")
        .auto_shrink(false);
    if std::mem::take(&mut gallery.scroll_to_top) {
        scroll = scroll.vertical_scroll_offset(0.0);
    }
    scroll.show_viewport(&mut child, |ui, viewport| {
        let height = GRID_TOP + grid.height(count) + 40.0;
        ui.set_min_size(egui::vec2(area.width(), height));
        let origin = ui.min_rect().min + egui::vec2(0.0, GRID_TOP);
        let visible = grid.visible(count, (viewport.top() - GRID_TOP).max(0.0), viewport.height());
        let ppp = ui.ctx().pixels_per_point();
        for index in visible {
            let Some(entry) = gallery.library.cards().get(index).cloned() else {
                break;
            };
            let thumb_rect = grid.thumb_rect(origin, index);
            let caption = grid.card_rect(origin, index);
            card::show(ui, gallery, &entry, thumb_rect, caption, ppp, actions);
        }
    });
}

/// Aucun projet : invitation centrée.
fn empty_state(ui: &mut egui::Ui, area: egui::Rect) {
    let center = area.center() - egui::vec2(0.0, 40.0);
    let painter = ui.painter();
    let glyph = egui::Rect::from_center_size(center - egui::vec2(0.0, 52.0), egui::vec2(32.0, 32.0));
    icons::paint(painter, glyph, Icon::Gallery, theme::TEXT_FAINT);
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        "No projects yet",
        theme::semibold(24.0),
        theme::TEXT,
    );
    let mut job = egui::text::LayoutJob::simple(
        "Tap + to start from the mannequin or an empty project,\nor Import a 3D model from the Files app."
            .to_owned(),
        egui::FontId::proportional(16.0),
        theme::TEXT_DIM,
        460.0,
    );
    job.halign = egui::Align::Center;
    let hint = painter.layout_job(job);
    painter.galley(center + egui::vec2(0.0, 26.0), hint, theme::TEXT_DIM);
}
