//! Interrupteur façon iOS (bleu d'accent quand il est activé) et ligne de
//! réglage (libellé, description, interrupteur), toute la ligne étant
//! touchable.

use egui::{Align2, Color32, CornerRadius, FontId, Rect, Response, Sense, Ui, pos2, vec2};

use super::{Motion, anim};
use crate::theme::{ACCENT, TEXT_DIM, TEXT_LIST, WIDGET_HOVER, mix};

pub(crate) const SIZE: egui::Vec2 = egui::Vec2::new(51.0, 31.0);

pub(crate) fn paint(ui: &Ui, rect: Rect, on: f32, press: f32) {
    let painter = ui.painter();
    let track = mix(WIDGET_HOVER, ACCENT, on);
    painter.rect_filled(rect, CornerRadius::same(16), track);
    let d = 27.0;
    let stretch = 5.0 * press;
    let x = rect.left() + 2.0 + on * (rect.width() - d - 4.0 - stretch);
    let knob = Rect::from_min_size(pos2(x, rect.top() + 2.0), vec2(d + stretch, d));
    painter.rect_filled(knob, CornerRadius::same(14), Color32::WHITE);
}

/// Interrupteur seul ; `changed()` quand il bascule.
pub fn switch(ui: &mut Ui, value: &mut bool) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(SIZE, Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let on = anim(ui, response.id.with("on"), *value);
        let motion = Motion::of(ui, &response);
        paint(ui, rect, on, motion.press);
    }
    response
}

/// Ligne de réglage hors liste groupée : libellé de 17 pt, description de
/// 13 pt et interrupteur à droite.
pub fn switch_row(ui: &mut Ui, value: &mut bool, label: &str, description: &str) -> Response {
    let width = ui.available_width().max(220.0);
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, 58.0), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let painter = ui.painter();
        let left = rect.left();
        painter.text(
            pos2(left, rect.center().y + 1.0),
            Align2::LEFT_BOTTOM,
            label,
            FontId::proportional(17.0),
            TEXT_LIST,
        );
        painter.text(
            pos2(left, rect.center().y + 3.0),
            Align2::LEFT_TOP,
            description,
            FontId::proportional(13.0),
            TEXT_DIM,
        );
        let switch = Align2::RIGHT_CENTER.anchor_size(rect.right_center(), SIZE);
        let on = anim(ui, response.id.with("on"), *value);
        paint(ui, switch, on, motion.press);
    }
    response
}
