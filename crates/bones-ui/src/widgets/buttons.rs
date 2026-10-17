//! Boutons : bouton d'outil carré, cellule de boîte à outils (icône +
//! libellé), bouton texte de la barre supérieure.
//!
//! L'état actif reprend la couleur de la famille de l'icône (comme Nomad
//! Sculpt) : tuile teintée, liseré lumineux et halo.

use egui::{Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, vec2};

use super::{Motion, anim, icon_tint};
use crate::draw;
use crate::icons::{self, Icon};
use crate::theme::{ACCENT, TEXT, TEXT_DIM, TEXT_FAINT, WIDGET, WIDGET_HOVER, mix, semibold};

/// Fond d'une tuile selon le survol, la pression et l'activation.
fn tile(ui: &Ui, rect: Rect, radius: u8, motion: &Motion, active: f32, tone: Color32) {
    let painter = ui.painter();
    let hover = Color32::from_white_alpha((10.0 + 14.0 * motion.press) as u8);
    if motion.hover > 0.0 || motion.press > 0.0 {
        painter.rect_filled(rect, radius, hover.gamma_multiply(motion.hover.max(motion.press)));
    }
    if active > 0.0 {
        draw::glow(
            painter,
            rect.center(),
            rect.width() * 0.75,
            tone.gamma_multiply(0.22 * active),
        );
        let fill = Color32::from_rgba_unmultiplied(tone.r(), tone.g(), tone.b(), (52.0 * active) as u8);
        painter.rect_filled(rect, radius, fill);
        let ring = Color32::from_rgba_unmultiplied(tone.r(), tone.g(), tone.b(), (210.0 * active) as u8);
        painter.rect_stroke(
            rect.shrink(0.75),
            radius,
            Stroke::new(1.5, ring),
            StrokeKind::Inside,
        );
        draw::sheen(painter, rect.shrink(1.5), radius.saturating_sub(1));
    }
}

/// Bouton d'outil carré 44 × 44 pt, icône en couleur de 26 pt.
pub fn tool_button(ui: &mut Ui, icon: Icon, active: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        tile(ui, rect, 12, &motion, on, icon.tone());
        let size = 26.0 * (1.0 - 0.08 * motion.press);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(rect.center(), vec2(size, size)),
            icon,
            icon_tint(ui),
        );
    }
    response.on_hover_text(hint)
}

/// Cellule de boîte à outils façon Nomad Sculpt : icône de 28 pt et
/// libellé de 11 pt dessous.
pub fn tool_cell(ui: &mut Ui, icon: Icon, label: &str, active: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(58.0, 62.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        tile(ui, rect, 14, &motion, on, icon.tone());
        let size = 28.0 * (1.0 - 0.08 * motion.press);
        let center = rect.center_top() + vec2(0.0, 24.0);
        icons::paint(
            ui.painter(),
            Rect::from_center_size(center, vec2(size, size)),
            icon,
            icon_tint(ui),
        );
        let color = if !ui.is_enabled() {
            TEXT_FAINT
        } else {
            mix(TEXT_DIM, TEXT, on.max(motion.hover * 0.5))
        };
        let font = if active {
            semibold(11.0)
        } else {
            FontId::proportional(11.0)
        };
        let galley = ui.painter().layout_no_wrap(label.to_owned(), font, color);
        let pos = rect.center_bottom() - vec2(galley.size().x * 0.5, galley.size().y + 7.0);
        ui.painter().galley(pos, galley, color);
    }
    response.on_hover_text(label)
}

/// Bouton texte de la barre supérieure (« Galerie ») : 17 pt, en accent
/// lorsqu'il est actif.
pub fn text_button(ui: &mut Ui, label: &str, active: bool) -> Response {
    let font = semibold(17.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), TEXT);
    let size = vec2(galley.size().x + 22.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let painter = ui.painter();
        let bg = Color32::from_white_alpha((14.0 * motion.hover.max(on) + 10.0 * motion.press) as u8);
        painter.rect_filled(rect, CornerRadius::same(12), bg);
        let color = mix(TEXT, ACCENT, on);
        let galley = painter.layout_no_wrap(label.to_owned(), font, color);
        painter.galley(rect.center() - galley.size() * 0.5, galley, color);
    }
    response
}

/// Bouton pilule (icône facultative + libellé). `primary` : dégradé
/// d'accent ; sinon fond neutre.
pub fn pill_button(ui: &mut Ui, icon: Option<Icon>, label: &str, primary: bool) -> Response {
    let font = semibold(15.0);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), TEXT);
    let icon_w = if icon.is_some() { 26.0 } else { 0.0 };
    let size = vec2(galley.size().x + icon_w + 36.0, 42.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let painter = ui.painter();
        let radius = CornerRadius::same(21);
        let rect = rect.shrink(1.5 * motion.press);
        let enabled = ui.is_enabled();
        if primary {
            draw::glow(
                painter,
                rect.center(),
                rect.width() * 0.55,
                ACCENT.gamma_multiply(0.18 + 0.12 * motion.hover),
            );
            draw::accent_fill(painter, rect, radius, if enabled { 1.0 } else { 0.35 });
        } else {
            let fill = mix(WIDGET, WIDGET_HOVER, motion.hover.max(motion.press));
            painter.rect_filled(
                rect,
                radius,
                if enabled { fill } else { fill.gamma_multiply(0.5) },
            );
        }
        draw::sheen(painter, rect, radius);
        let color = if enabled { Color32::WHITE } else { TEXT_FAINT };
        let mut x = rect.center().x - (galley.size().x + icon_w) * 0.5;
        if let Some(icon) = icon {
            let r = Rect::from_center_size(egui::pos2(x + 10.0, rect.center().y), vec2(20.0, 20.0));
            icons::paint(painter, r, icon, icon_tint(ui));
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

/// Petit séparateur vertical entre deux groupes d'une barre horizontale.
pub fn separator_dot(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(9.0, 28.0), Sense::hover());
    ui.painter().line_segment(
        [
            rect.center_top() + vec2(0.0, 4.0),
            rect.center_bottom() - vec2(0.0, 4.0),
        ],
        Stroke::new(1.0, Color32::from_white_alpha(28)),
    );
}
