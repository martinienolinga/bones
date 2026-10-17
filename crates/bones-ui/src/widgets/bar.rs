//! Commandes de la barre du haut et du bandeau latéral, à la Procreate :
//! glyphes dans des puits ronds, outils sans fond (bleus quand actifs),
//! petit carré cerclé, onglets glyphe + libellé.

use egui::{CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, vec2};

use super::{Motion, anim, glyph_color};
use crate::icons::{self, Icon};
use crate::theme::{ACCENT, ICON, TEXT, TEXT_DIM, WELL, WIDGET, mix};

/// Glyphe centré dans un puits rond de 32 pt (zone tactile 44 pt) ; actif :
/// puits bleu plein et glyphe blanc.
pub fn well_button(ui: &mut Ui, icon: Icon, active: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let idle = mix(WELL, WIDGET, motion.hover.max(motion.press));
        let radius = 16.0 * (1.0 - 0.04 * motion.press);
        ui.painter()
            .circle_filled(rect.center(), radius, mix(idle, ACCENT, on));
        let color = mix(ICON, TEXT, on);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(18.0, 18.0)),
            icon,
            color,
        );
    }
    response.on_hover_text(hint)
}

/// Outil de la barre (glyphe de 24 pt sans fond, zone 48 × 44 pt) : blanc
/// au repos, bleu actif, gris désactivé.
pub fn bar_tool(ui: &mut Ui, icon: Icon, active: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(48.0, 44.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let size = 24.0 * (1.0 - 0.08 * motion.press);
        let color = glyph_color(ui, on).gamma_multiply(1.0 - 0.3 * motion.press);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(size, size)),
            icon,
            color,
        );
    }
    response.on_hover_text(hint)
}

/// Petit carré cerclé de 20 pt (zone 44 pt) du bandeau latéral.
pub fn square_button(ui: &mut Ui, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 40.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let square = Rect::from_center_size(rect.center(), vec2(20.0, 20.0));
        let painter = ui.painter();
        if motion.press > 0.0 {
            let fill = egui::Color32::from_white_alpha((60.0 * motion.press) as u8);
            painter.rect_filled(square, CornerRadius::same(5), fill);
        }
        painter.rect_stroke(
            square,
            CornerRadius::same(5),
            Stroke::new(1.5, glyph_color(ui, 0.0)),
            StrokeKind::Inside,
        );
    }
    response.on_hover_text(hint)
}

/// Onglet : glyphe de 22 pt au-dessus d'un libellé de 12 pt ; gris au
/// repos, bleu actif.
pub fn tab(ui: &mut Ui, icon: Icon, label: &str, active: bool) -> Response {
    let font = FontId::proportional(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), TEXT_DIM);
    let width = (galley.size().x + 16.0).max(64.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 54.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let color = mix(TEXT_DIM, ACCENT, on).gamma_multiply(1.0 - 0.3 * motion.press);
        let glyph = Rect::from_center_size(rect.center_top() + vec2(0.0, 17.0), vec2(22.0, 22.0));
        icons::paint(ui.painter(), glyph, icon, color);
        let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
        let pos = rect.center_bottom() - vec2(galley.size().x * 0.5, galley.size().y + 3.0);
        ui.painter().galley(pos, galley, color);
    }
    response
}
