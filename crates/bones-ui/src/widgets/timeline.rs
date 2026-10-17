//! Frise de clés (timeline) : graduations, losanges de clés, tête de
//! lecture ; un tap ou un glissement déplace l'image courante.

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

use crate::theme::{ACCENT, TEXT, TEXT_DIM, WIDGET};

const PAD: f32 = 14.0;
/// Pas majeurs possibles, en images.
const STEPS: [u32; 12] = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000];

/// Image entière sous l'abscisse `x` d'une frise commençant en `left`.
pub(crate) fn frame_at(x: f32, left: f32, width: f32, count: u32) -> f32 {
    let last = count.max(2) - 1;
    if width <= 0.0 {
        return 0.0;
    }
    ((x - left) / width * last as f32).round().clamp(0.0, last as f32)
}

/// Pas majeur (libellés) et mineur (petites graduations) selon la densité.
pub(crate) fn tick_steps(px_per_frame: f32) -> (u32, Option<u32>) {
    let major = STEPS
        .into_iter()
        .find(|s| *s as f32 * px_per_frame >= 48.0)
        .unwrap_or(10_000);
    let minor = [major / 5, major / 2]
        .into_iter()
        .find(|m| *m >= 1 && major % m == 0 && *m as f32 * px_per_frame >= 8.0);
    (major, minor)
}

/// Frise de `frame_count` images avec des clés aux images `keys`.
/// La réponse est marquée `changed()` quand `*frame` change.
pub fn key_strip(ui: &mut Ui, frame: &mut f32, frame_count: u32, keys: &[u32], height: f32) -> Response {
    let width = ui.available_width().max(120.0);
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
    let count = frame_count.max(2);
    let last = (count - 1) as f32;
    let (left, span) = (rect.left() + PAD, (rect.width() - 2.0 * PAD).max(1.0));

    if (response.is_pointer_button_down_on() || response.dragged() || response.clicked())
        && let Some(pos) = response.interact_pointer_pos()
    {
        let f = frame_at(pos.x, left, span, count);
        if f != *frame {
            *frame = f;
            response.mark_changed();
        }
    }

    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    let x_of = |f: f32| left + f / last * span;
    painter.rect_filled(rect, CornerRadius::same(8), WIDGET);

    // Graduations et numéros d'image.
    let (major, minor) = tick_steps(span / last);
    let step = minor.unwrap_or(major);
    let (tick_top, mid) = (rect.top() + 20.0, rect.center().y + 6.0);
    let grid = Color32::from_white_alpha(46);
    for f in (0..count).step_by(step as usize) {
        let x = x_of(f as f32);
        if f % major == 0 {
            painter.line_segment(
                [pos2(x, tick_top), pos2(x, rect.bottom() - 6.0)],
                Stroke::new(1.0, grid),
            );
            painter.text(
                pos2(x, rect.top() + 4.0),
                Align2::CENTER_TOP,
                f,
                FontId::proportional(11.0),
                TEXT_DIM,
            );
        } else {
            painter.line_segment([pos2(x, mid - 4.0), pos2(x, mid + 4.0)], Stroke::new(1.0, grid));
        }
    }

    // Clés en losanges.
    let current = frame.round() as u32;
    for &k in keys.iter().filter(|k| **k < count) {
        let c = pos2(x_of(k as f32), mid);
        let stroke = if k == current {
            Stroke::new(1.5, Color32::WHITE)
        } else {
            Stroke::NONE
        };
        painter.add(Shape::convex_polygon(diamond(c, 6.5), ACCENT, stroke));
    }

    // Tête de lecture et son numéro.
    let x = x_of(frame.clamp(0.0, last));
    painter.line_segment(
        [pos2(x, tick_top - 2.0), pos2(x, rect.bottom())],
        Stroke::new(2.0, TEXT),
    );
    let label = painter.layout_no_wrap(format!("{current}"), FontId::proportional(11.0), Color32::WHITE);
    let pill = Align2::CENTER_BOTTOM.anchor_size(pos2(x, tick_top), label.size() + vec2(10.0, 4.0));
    painter.rect_filled(pill, CornerRadius::same(6), ACCENT);
    painter.galley(pill.center() - label.size() * 0.5, label, Color32::WHITE);
    response
}

fn diamond(c: Pos2, r: f32) -> Vec<Pos2> {
    vec![
        pos2(c.x, c.y - r),
        pos2(c.x + r, c.y),
        pos2(c.x, c.y + r),
        pos2(c.x - r, c.y),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_mapping_rounds_and_clamps() {
        assert_eq!(frame_at(0.0, 0.0, 100.0, 11), 0.0);
        assert_eq!(frame_at(54.0, 0.0, 100.0, 11), 5.0);
        assert_eq!(frame_at(500.0, 0.0, 100.0, 11), 10.0);
        assert_eq!(frame_at(-5.0, 0.0, 100.0, 11), 0.0);
    }

    #[test]
    fn ticks_stay_readable() {
        assert_eq!(tick_steps(10.0), (5, Some(1)));
        let (major, minor) = tick_steps(0.5);
        assert!(major as f32 * 0.5 >= 48.0);
        assert!(minor.is_none_or(|m| m as f32 * 0.5 >= 8.0 && major % m == 0));
    }
}
