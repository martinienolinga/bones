//! Widgets tactiles : boutons d'outils (Procreate), cellules de boîte à
//! outils (Nomad Sculpt), contrôle segmenté, curseurs verticaux, frise de
//! clés.

mod segmented;
mod slider;
mod timeline;

pub use segmented::segmented;
pub use slider::{vertical_slider, vertical_slider_with_icon};
pub use timeline::key_strip;

use egui::{Color32, CornerRadius, FontId, Rect, Response, Sense, Ui, vec2};

use crate::icons::{self, Icon};
use crate::theme::{ACCENT, ACCENT_SOFT, TEXT, TEXT_DIM};

/// Couleur d'un élément selon son état (désactivé : atténué).
fn ink(ui: &Ui, color: Color32) -> Color32 {
    if ui.is_enabled() {
        color
    } else {
        color.gamma_multiply(0.35)
    }
}

/// Retour visuel de pression / survol sur un fond transparent.
fn feedback(response: &Response) -> Color32 {
    if response.is_pointer_button_down_on() {
        Color32::from_white_alpha(36)
    } else if response.hovered() {
        Color32::from_white_alpha(16)
    } else {
        Color32::TRANSPARENT
    }
}

/// Bouton d'outil carré 44 × 44 pt, icône de 26 pt ; rempli d'accent
/// quand il est actif.
pub fn tool_button(ui: &mut Ui, icon: Icon, active: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if active { ACCENT } else { feedback(&response) };
        painter.rect_filled(rect, CornerRadius::same(10), fill);
        let color = ink(ui, if active { Color32::WHITE } else { TEXT });
        icons::paint(
            painter,
            Rect::from_center_size(rect.center(), vec2(26.0, 26.0)),
            icon,
            color,
        );
    }
    response.on_hover_text(hint)
}

/// Cellule de la boîte à outils de droite façon Nomad Sculpt : icône de
/// 24 pt et libellé de 11 pt dessous.
pub fn tool_cell(ui: &mut Ui, icon: Icon, label: &str, active: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(52.0, 60.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if active { ACCENT_SOFT } else { feedback(&response) };
        painter.rect_filled(rect, CornerRadius::same(8), fill);
        let icon_rect = Rect::from_center_size(rect.center_top() + vec2(0.0, 22.0), vec2(24.0, 24.0));
        icons::paint(
            painter,
            icon_rect,
            icon,
            ink(ui, if active { Color32::WHITE } else { TEXT }),
        );
        let text_color = ink(ui, if active { TEXT } else { TEXT_DIM });
        let galley = painter.layout_no_wrap(label.to_owned(), FontId::proportional(11.0), text_color);
        let pos = rect.center_bottom() - vec2(galley.size().x * 0.5, galley.size().y + 6.0);
        painter.galley(pos, galley, text_color);
    }
    response.on_hover_text(label)
}

/// Bouton texte de la barre supérieure (« Galerie », « Actions »…) : texte
/// seul de 17 pt, en accent lorsqu'il est actif.
pub fn text_button(ui: &mut Ui, label: &str, active: bool) -> Response {
    let color = ink(ui, if active { ACCENT } else { TEXT });
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(17.0), color);
    let size = vec2(galley.size().x + 16.0, galley.size().y.max(24.0) + 12.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(8), feedback(&response));
        painter.galley(rect.center() - galley.size() * 0.5, galley, color);
    }
    response
}

/// Petit point de séparation entre deux groupes d'une barre horizontale.
pub fn separator_dot(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 24.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 1.6, TEXT_DIM);
}
