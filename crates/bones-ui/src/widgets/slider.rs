//! Curseur vertical de la barre latérale, façon Procreate : glissement
//! relatif (le pouce ne saute pas sous le doigt) et bulle de valeur.

use std::ops::RangeInclusive;

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, LayerId, Order, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    pos2, vec2,
};

use crate::icons::{self, Icon};
use crate::theme::{ACCENT, PANEL, PANEL_BORDER, TEXT, THUMB, WIDGET};

const WIDTH: f32 = 36.0;
const HEIGHT: f32 = 160.0;
const ICON_SPACE: f32 = 30.0;
const THUMB_SIZE: egui::Vec2 = egui::Vec2::new(30.0, 16.0);

/// Nouvelle valeur après un glissement de `delta_up` points vers le haut sur
/// une piste de `height` points couvrant `range`.
pub(crate) fn apply_drag(value: f32, delta_up: f32, height: f32, range: &RangeInclusive<f32>) -> f32 {
    let (lo, hi) = (*range.start(), *range.end());
    if hi <= lo || height <= 0.0 {
        return value.clamp(lo.min(hi), hi.max(lo));
    }
    let t = ((value - lo) / (hi - lo) + delta_up / height).clamp(0.0, 1.0);
    lo + t * (hi - lo)
}

/// Texte de la bulle : entier si les bornes le sont, pourcentage sinon.
pub(crate) fn format_value(value: f32, range: &RangeInclusive<f32>) -> String {
    let (lo, hi) = (*range.start(), *range.end());
    if lo.fract() == 0.0 && hi.fract() == 0.0 && hi - lo >= 1.0 {
        format!("{}", value.round() as i64)
    } else if hi > lo {
        format!("{:.0} %", (value - lo) / (hi - lo) * 100.0)
    } else {
        format!("{value:.2}")
    }
}

/// Curseur vertical 36 × 160 pt (remplissage depuis le bas).
pub fn vertical_slider(ui: &mut Ui, value: &mut f32, range: RangeInclusive<f32>, hint: &str) -> Response {
    slider(ui, value, range, None, hint)
}

/// Variante surmontée d'une icône, comme les curseurs rayon / intensité de
/// Nomad Sculpt.
pub fn vertical_slider_with_icon(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    icon: Icon,
    hint: &str,
) -> Response {
    slider(ui, value, range, Some(icon), hint)
}

fn slider(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    icon: Option<Icon>,
    hint: &str,
) -> Response {
    let extra = if icon.is_some() { ICON_SPACE } else { 0.0 };
    let (rect, mut response) = ui.allocate_exact_size(vec2(WIDTH, HEIGHT + extra), Sense::drag());
    let track = Rect::from_min_max(pos2(rect.left(), rect.top() + extra), rect.max);
    let usable = track.height() - THUMB_SIZE.y - 8.0;

    if response.dragged() {
        let new = apply_drag(*value, -response.drag_delta().y, usable, &range);
        if new != *value {
            *value = new;
            response.mark_changed();
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if let Some(icon) = icon {
            let r = Rect::from_center_size(
                pos2(rect.center().x, rect.top() + ICON_SPACE * 0.45),
                vec2(22.0, 22.0),
            );
            icons::paint(painter, r, icon, TEXT);
        }
        let radius = CornerRadius::same(10);
        painter.rect_filled(track, radius, WIDGET);
        let t = {
            let (lo, hi) = (*range.start(), *range.end());
            if hi > lo {
                ((*value - lo) / (hi - lo)).clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        let thumb_y = track.bottom() - 4.0 - THUMB_SIZE.y * 0.5 - t * usable;
        let filled = Rect::from_min_max(pos2(track.left(), thumb_y), track.max);
        painter.rect_filled(filled, radius, ACCENT.gamma_multiply(0.6));
        painter.rect_stroke(track, radius, Stroke::new(1.0, PANEL_BORDER), StrokeKind::Inside);
        let thumb = Rect::from_center_size(pos2(track.center().x, thumb_y), THUMB_SIZE);
        let thumb_color = if response.dragged() { Color32::WHITE } else { THUMB };
        painter.rect_filled(thumb, CornerRadius::same(8), thumb_color);
    }

    if response.dragged() {
        bubble(
            ui,
            response.id,
            rect,
            &format!("{hint} · {}", format_value(*value, &range)),
        );
    }
    response.on_hover_text(hint)
}

/// Bulle de valeur affichée à droite du curseur pendant le glissement.
fn bubble(ui: &Ui, id: Id, slider: Rect, text: &str) {
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Tooltip, id.with("bubble")));
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(14.0), TEXT);
    let size = galley.size() + vec2(20.0, 12.0);
    let anchor = pos2(slider.right() + 12.0, slider.center().y);
    let rect = Align2::LEFT_CENTER.anchor_size(anchor, size);
    painter.rect_filled(rect, CornerRadius::same(8), PANEL);
    painter.galley(rect.center() - galley.size() * 0.5, galley, TEXT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_is_relative_and_clamped() {
        let r = 0.0..=10.0;
        assert_eq!(apply_drag(5.0, 0.0, 100.0, &r), 5.0);
        assert!((apply_drag(5.0, 20.0, 100.0, &r) - 7.0).abs() < 1e-5);
        assert_eq!(apply_drag(5.0, 1000.0, 100.0, &r), 10.0);
        assert_eq!(apply_drag(5.0, -1000.0, 100.0, &r), 0.0);
        assert_eq!(apply_drag(3.0, 50.0, 100.0, &(2.0..=2.0)), 2.0);
    }

    #[test]
    fn value_formatting() {
        assert_eq!(format_value(3.4, &(1.0..=6.0)), "3");
        assert_eq!(format_value(0.25, &(0.0..=0.5)), "50 %");
    }
}
