//! Curseur vertical façon Procreate : piste grise arrondie sans
//! remplissage, pouce en pilule grise, glissement relatif et bulle de valeur
//! pendant le geste.

use std::ops::RangeInclusive;

use egui::{Align2, CornerRadius, FontId, Id, LayerId, Order, Rect, Response, Sense, Ui, pos2, vec2};

use super::anim;
use crate::icons::{self, Icon};
use crate::theme::{ICON, PANEL, TEXT, TEXT_DIM, THUMB, WIDGET, semibold, small_shadow};

const WIDTH: f32 = 36.0;
const HEIGHT: f32 = 200.0;
/// Hauteur réduite pour empiler plusieurs réglages (poids, rayon, force).
const COMPACT_HEIGHT: f32 = 150.0;
const ICON_SPACE: f32 = 32.0;

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

pub(crate) fn fraction(value: f32, range: &RangeInclusive<f32>) -> f32 {
    let (lo, hi) = (*range.start(), *range.end());
    if hi > lo {
        ((value - lo) / (hi - lo)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Curseur vertical 36 × 200 pt.
pub fn vertical_slider(ui: &mut Ui, value: &mut f32, range: RangeInclusive<f32>, hint: &str) -> Response {
    slider(ui, value, range, None, hint, HEIGHT, None)
}

/// Variante surmontée d'une icône (curseurs rayon / intensité de Nomad).
pub fn vertical_slider_with_icon(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    icon: Icon,
    hint: &str,
) -> Response {
    slider(ui, value, range, Some(icon), hint, HEIGHT, None)
}

/// Variante compacte à icône, dont la bulle affiche `decimals` décimales
/// (réglages de pinceau empilés : poids « 1.000 », rayon, force).
pub fn vertical_slider_compact(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    icon: Icon,
    hint: &str,
    decimals: usize,
) -> Response {
    slider(ui, value, range, Some(icon), hint, COMPACT_HEIGHT, Some(decimals))
}

fn slider(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    icon: Option<Icon>,
    hint: &str,
    height: f32,
    decimals: Option<usize>,
) -> Response {
    let extra = if icon.is_some() { ICON_SPACE } else { 0.0 };
    let (rect, mut response) = ui.allocate_exact_size(vec2(WIDTH, height + extra), Sense::drag());
    let track = Rect::from_min_max(pos2(rect.left(), rect.top() + extra), rect.max);
    let usable = track.height() - 26.0;

    if response.dragged() {
        let new = apply_drag(*value, -response.drag_delta().y, usable, &range);
        if new != *value {
            *value = new;
            response.mark_changed();
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let grab = anim(ui, response.id.with("grab"), response.dragged());
        if let Some(icon) = icon {
            let r = Rect::from_center_size(
                pos2(rect.center().x, rect.top() + ICON_SPACE * 0.42),
                vec2(22.0, 22.0),
            );
            icons::paint(painter, r, icon, ICON);
        }
        let radius = CornerRadius::same((WIDTH * 0.5) as u8);
        painter.rect_filled(track, radius, WIDGET);
        let t = fraction(*value, &range);
        let thumb_y = track.bottom() - 13.0 - t * usable;
        let size = vec2(30.0, 18.0) * (1.0 + 0.08 * grab);
        let thumb = Rect::from_center_size(pos2(track.center().x, thumb_y), size);
        painter.rect_filled(thumb, CornerRadius::same(9), THUMB);
    }

    if response.dragged() {
        let text = match decimals {
            Some(n) => format!("{:.*}", n, *value),
            None => format_value(*value, &range),
        };
        bubble(ui, response.id, rect, hint, &text);
    }
    response.on_hover_text(hint)
}

/// Bulle de valeur affichée à droite du curseur pendant le glissement.
fn bubble(ui: &Ui, id: Id, slider: Rect, label: &str, value: &str) {
    let painter = ui
        .ctx()
        .layer_painter(LayerId::new(Order::Tooltip, id.with("bubble")));
    let big = painter.layout_no_wrap(value.to_owned(), semibold(20.0), TEXT);
    let small = painter.layout_no_wrap(label.to_owned(), FontId::proportional(12.0), TEXT_DIM);
    let size = vec2(
        big.size().x.max(small.size().x) + 28.0,
        big.size().y + small.size().y + 18.0,
    );
    let rect = Align2::LEFT_CENTER.anchor_size(pos2(slider.right() + 14.0, slider.center().y), size);
    painter.add(small_shadow().as_shape(rect, CornerRadius::same(14)));
    painter.rect_filled(rect, CornerRadius::same(14), PANEL);
    let top = rect.top() + 9.0;
    painter.galley(pos2(rect.center().x - big.size().x * 0.5, top), big.clone(), TEXT);
    painter.galley(
        pos2(rect.center().x - small.size().x * 0.5, top + big.size().y),
        small,
        TEXT_DIM,
    );
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
