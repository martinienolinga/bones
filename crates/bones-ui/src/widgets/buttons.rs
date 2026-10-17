//! Boutons : bouton d'outil carré, cellule de boîte à outils (glyphe +
//! libellé), bouton texte et bouton pilule.
//!
//! Comme dans Procreate, l'état actif ne se lit qu'à la couleur : glyphe
//! (et libellé) en bleu d'accent, sans pastille ni fond.

use egui::{Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, Ui, vec2};

use super::{Motion, anim, glyph_color};
use crate::icons::{self, Icon};
use crate::theme::{
    ACCENT, ACCENT_PRESSED, DISABLED, HAIRLINE, TEXT, TEXT_DIM, WIDGET, WIDGET_HOVER, mix, semibold,
};

/// Léger fond de retour tactile (survol, pression).
fn feedback(ui: &Ui, rect: Rect, radius: u8, motion: &Motion) {
    let t = motion.hover.max(motion.press);
    if t > 0.0 {
        let alpha = 8.0 * motion.hover + 10.0 * motion.press;
        ui.painter()
            .rect_filled(rect, radius, Color32::from_white_alpha(alpha as u8));
    }
}

/// Bouton d'outil carré 44 × 44 pt, glyphe de 24 pt ; actif = glyphe bleu.
pub fn tool_button(ui: &mut Ui, icon: Icon, active: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        feedback(ui, rect, 12, &motion);
        let size = 24.0 * (1.0 - 0.08 * motion.press);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(size, size)),
            icon,
            glyph_color(ui, on),
        );
    }
    response.on_hover_text(hint)
}

/// Cellule de boîte à outils : glyphe de 24 pt et libellé de 11 pt gris ;
/// active = glyphe et libellé bleus.
pub fn tool_cell(ui: &mut Ui, icon: Icon, label: &str, active: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        feedback(ui, rect, 12, &motion);
        let size = 24.0 * (1.0 - 0.08 * motion.press);
        let center = rect.center_top() + vec2(0.0, 21.0);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(center, vec2(size, size)),
            icon,
            glyph_color(ui, on),
        );
        let color = if ui.is_enabled() {
            mix(TEXT_DIM, ACCENT, on)
        } else {
            DISABLED
        };
        let font = if active {
            semibold(11.0)
        } else {
            FontId::proportional(11.0)
        };
        let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
        let pos = rect.center_bottom() - vec2(galley.size().x * 0.5, galley.size().y + 4.0);
        ui.painter().galley(pos, galley, color);
    }
    response.on_hover_text(label)
}

/// Bouton texte de la barre du haut (« Gallery ») : 17 pt, bleu lorsqu'il
/// est actif.
pub fn text_button(ui: &mut Ui, label: &str, active: bool) -> Response {
    let font = FontId::proportional(17.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), TEXT);
    let size = vec2(galley.size().x + 16.0, 44.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let color = mix(TEXT, ACCENT, on).gamma_multiply(1.0 - 0.35 * motion.press);
        let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
        ui.painter()
            .galley(rect.center() - galley.size() * 0.5, galley, color);
    }
    response
}

/// Bouton pilule (glyphe facultatif + libellé de 15 pt). `primary` : aplat
/// d'accent ; sinon fond gris.
pub fn pill_button(ui: &mut Ui, icon: Option<Icon>, label: &str, primary: bool) -> Response {
    let font = semibold(15.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), TEXT);
    let icon_w = if icon.is_some() { 26.0 } else { 0.0 };
    let size = vec2(galley.size().x + icon_w + 32.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let painter = ui.painter();
        let radius = CornerRadius::same(20);
        let enabled = ui.is_enabled();
        let t = motion.hover.max(motion.press);
        let fill = if primary {
            mix(ACCENT, ACCENT_PRESSED, t)
        } else {
            mix(WIDGET, WIDGET_HOVER, t)
        };
        painter.rect_filled(
            rect,
            radius,
            if enabled { fill } else { fill.gamma_multiply(0.45) },
        );
        let color = if enabled { TEXT } else { DISABLED };
        let mut x = rect.center().x - (galley.size().x + icon_w) * 0.5;
        if let Some(icon) = icon {
            let r = Rect::from_center_size(egui::pos2(x + 9.0, rect.center().y), vec2(18.0, 18.0));
            icons::paint(painter, r, icon, color);
            x += icon_w;
        }
        let galley = painter.layout_no_wrap(label.to_owned(), font, color);
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y * 0.5),
            galley,
            color,
        );
    }
    response
}

/// Filet vertical entre deux groupes d'une barre horizontale.
pub fn separator_dot(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(9.0, 28.0), Sense::hover());
    ui.painter().line_segment(
        [
            rect.center_top() + vec2(0.0, 4.0),
            rect.center_bottom() - vec2(0.0, 4.0),
        ],
        Stroke::new(1.0, HAIRLINE),
    );
}
