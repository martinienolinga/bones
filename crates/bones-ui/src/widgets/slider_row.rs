//! Réglage à curseur fin, à la Brush Studio de Procreate : libellé de
//! 17 pt à gauche, pastille de valeur à droite, et dessous une piste de
//! 3 pt (partie remplie en bleu) avec un pouce blanc en pilule.

use std::ops::RangeInclusive;

use egui::{Align, CornerRadius, FontId, Layout, Rect, Response, Sense, Ui, pos2, vec2};

use super::slider::{format_value, fraction};
use super::{anim, value_chip};
use crate::theme::{ACCENT, HAIRLINE, TEXT_LIST, small_shadow};

/// Curseur fin avec libellé et valeur affichée (`unit` après le nombre).
pub fn slider_row(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    label: &str,
    unit: &str,
) -> Response {
    let text = format!("{} {unit}", format_value(*value, &range));
    slider_row_with(ui, value, range, label, |ui, _| value_chip(ui, text.trim()))
}

/// Variante dont la pastille est fournie par `chip` (pastille modifiable,
/// unité particulière…). La réponse est `changed()` si l'un ou l'autre a
/// changé la valeur.
pub fn slider_row_with(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    label: &str,
    chip: impl FnOnce(&mut Ui, &mut f32) -> Response,
) -> Response {
    let width = ui.available_width().max(160.0);
    let chip_response = ui
        .allocate_ui_with_layout(vec2(width, 32.0), Layout::left_to_right(Align::Center), |ui| {
            ui.label(
                egui::RichText::new(label)
                    .font(FontId::proportional(17.0))
                    .color(TEXT_LIST),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| chip(ui, value))
                .inner
        })
        .inner;
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, 26.0), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), vec2(width - 22.0, 3.0));
    if ui.is_enabled()
        && let Some(pos) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked())
    {
        let t = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
        let new = *range.start() + t * (*range.end() - *range.start());
        if new != *value {
            *value = new;
            response.mark_changed();
        }
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(track, CornerRadius::same(2), HAIRLINE);
        let x = track.left() + fraction(*value, &range) * track.width();
        let fill = if ui.is_enabled() { ACCENT } else { HAIRLINE };
        painter.rect_filled(
            Rect::from_min_max(track.min, pos2(x, track.bottom())),
            CornerRadius::same(2),
            fill,
        );
        let grab = anim(ui, response.id.with("grab"), response.dragged());
        let thumb = Rect::from_center_size(pos2(x, track.center().y), vec2(22.0, 12.0) * (1.0 + 0.12 * grab));
        painter.add(small_shadow().as_shape(thumb, CornerRadius::same(6)));
        painter.rect_filled(thumb, CornerRadius::same(6), egui::Color32::WHITE);
    }
    if chip_response.changed() {
        response.mark_changed();
    }
    response
}
