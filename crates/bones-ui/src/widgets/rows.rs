//! Ligne de liste isolée (hors liste groupée) et titres de section des
//! fenêtres contextuelles.

use egui::{Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Ui, pos2, vec2};

use super::{Motion, glyph_color};
use crate::icons::{self, Icon};
use crate::theme::{TEXT, TEXT_DIM, TEXT_LIST, WIDGET_HOVER, semibold};

/// Titre de section (15 pt semi-gras, retrait de 16 pt).
pub fn section_title(ui: &mut Ui, text: &str) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(egui::RichText::new(text).font(semibold(15.0)).color(TEXT));
    });
    ui.add_space(2.0);
}

/// Note discrète sous une liste (13 pt gris, retrait de 16 pt).
pub fn note(ui: &mut Ui, text: &str) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 16,
            right: 16,
            top: 2,
            bottom: 2,
        })
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(13.0).color(TEXT_DIM));
        });
}

/// Ligne touchable : glyphe gris, titre, sous-titre et pastille à droite
/// (format de fichier…).
pub fn list_row(
    ui: &mut Ui,
    icon: Icon,
    title: &str,
    subtitle: Option<&str>,
    badge: Option<&str>,
) -> Response {
    let width = ui.available_width().max(200.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 54.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let painter = ui.painter();
        let bg = Color32::from_white_alpha((5.0 + 10.0 * motion.hover + 10.0 * motion.press) as u8);
        painter.rect_filled(rect, CornerRadius::same(12), bg);

        let tile = Rect::from_center_size(pos2(rect.left() + 27.0, rect.center().y), vec2(22.0, 22.0));
        icons::paint(painter, tile, icon, glyph_color(ui, 0.0));

        let mut right = rect.right() - 12.0;
        if let Some(badge) = badge {
            let galley = painter.layout_no_wrap(badge.to_owned(), semibold(11.0), TEXT);
            let pill = Align2::RIGHT_CENTER
                .anchor_size(pos2(right, rect.center().y), galley.size() + vec2(14.0, 8.0));
            painter.rect_filled(pill, CornerRadius::same(8), WIDGET_HOVER);
            painter.galley(pill.center() - galley.size() * 0.5, galley, TEXT);
            right = pill.left() - 8.0;
        }

        let left = tile.right() + 14.0;
        let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()));
        let painter = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        match subtitle {
            Some(sub) => {
                painter.text(
                    pos2(left, rect.center().y),
                    Align2::LEFT_BOTTOM,
                    title,
                    FontId::proportional(17.0),
                    TEXT_LIST,
                );
                painter.text(
                    pos2(left, rect.center().y + 2.0),
                    Align2::LEFT_TOP,
                    sub,
                    FontId::proportional(13.0),
                    TEXT_DIM,
                );
            }
            None => {
                painter.text(
                    pos2(left, rect.center().y),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(17.0),
                    TEXT_LIST,
                );
            }
        }
    }
    response
}
