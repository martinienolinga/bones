//! Lignes de liste (fichiers, formats d'export) et titres de section des
//! fenêtres contextuelles.

use egui::{Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Ui, pos2, vec2};

use super::{Motion, icon_tint};
use crate::icons::{self, Icon};
use crate::theme::{TEXT, TEXT_DIM, semibold};

/// Titre de section discret (13 pt semi-gras, gris).
pub fn section_title(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).font(semibold(13.0)).color(TEXT_DIM));
}

/// Ligne touchable : icône dans une tuile teintée, titre, sous-titre et
/// pastille à droite (format de fichier…).
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

        let tone = icon.tone();
        let tile = Rect::from_center_size(pos2(rect.left() + 29.0, rect.center().y), vec2(38.0, 38.0));
        let fill = Color32::from_rgba_unmultiplied(tone.r(), tone.g(), tone.b(), 46);
        painter.rect_filled(tile, CornerRadius::same(10), fill);
        icons::paint(painter, tile.shrink(7.0), icon, icon_tint(ui));

        let mut right = rect.right() - 12.0;
        if let Some(badge) = badge {
            let galley = painter.layout_no_wrap(badge.to_owned(), semibold(11.0), TEXT);
            let pill = Align2::RIGHT_CENTER
                .anchor_size(pos2(right, rect.center().y), galley.size() + vec2(14.0, 8.0));
            let fill = Color32::from_rgba_unmultiplied(tone.r(), tone.g(), tone.b(), 70);
            painter.rect_filled(pill, CornerRadius::same(8), fill);
            painter.galley(pill.center() - galley.size() * 0.5, galley, TEXT);
            right = pill.left() - 8.0;
        }

        let left = tile.right() + 12.0;
        let clip = Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()));
        let painter = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
        match subtitle {
            Some(sub) => {
                painter.text(
                    pos2(left, rect.center().y),
                    Align2::LEFT_BOTTOM,
                    title,
                    FontId::proportional(15.0),
                    TEXT,
                );
                painter.text(
                    pos2(left, rect.center().y + 2.0),
                    Align2::LEFT_TOP,
                    sub,
                    FontId::proportional(12.0),
                    TEXT_DIM,
                );
            }
            None => {
                painter.text(
                    pos2(left, rect.center().y),
                    Align2::LEFT_CENTER,
                    title,
                    FontId::proportional(15.0),
                    TEXT,
                );
            }
        }
    }
    response
}
