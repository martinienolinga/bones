//! Contrôle segmenté : une pilule en dégradé d'accent glisse sous le choix
//! courant (choix du mode, de l'interpolation…).

use egui::{Color32, CornerRadius, FontId, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::draw;
use crate::theme::{TEXT, TEXT_DIM, TEXT_FAINT, semibold};

const HEIGHT: f32 = 36.0;
const PAD: f32 = 15.0;
const INSET: f32 = 3.0;

/// Pilule à plusieurs segments ; la réponse est marquée `changed()` quand
/// la sélection change.
pub fn segmented(ui: &mut Ui, options: &[&str], selected: &mut usize) -> Response {
    let measure = FontId::proportional(14.0);
    let widths: Vec<f32> = options
        .iter()
        .map(|o| {
            let g = ui.painter().layout_no_wrap((*o).to_owned(), semibold(14.0), TEXT);
            let r = ui
                .painter()
                .layout_no_wrap((*o).to_owned(), measure.clone(), TEXT);
            g.size().x.max(r.size().x) + 2.0 * PAD
        })
        .collect();
    let total = widths.iter().sum::<f32>() + 2.0 * INSET;
    let (rect, mut response) = ui.allocate_exact_size(vec2(total, HEIGHT), Sense::click());

    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let mut x = rect.left() + INSET;
        for (i, w) in widths.iter().enumerate() {
            if pos.x >= x && pos.x < x + w && i != *selected {
                *selected = i;
                response.mark_changed();
            }
            x += w;
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = CornerRadius::same((HEIGHT * 0.5) as u8);
        painter.rect_filled(rect, radius, Color32::from_rgb(0x15, 0x15, 0x18));
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, Color32::from_white_alpha(14)),
            StrokeKind::Inside,
        );

        // Position animée de la pilule de sélection.
        let index = (*selected).min(widths.len().saturating_sub(1));
        let target_x: f32 = widths[..index].iter().sum();
        let ctx = ui.ctx();
        let x = ctx.animate_value_with_time(response.id.with("x"), target_x, 0.2);
        let w = ctx.animate_value_with_time(response.id.with("w"), widths[index], 0.2);
        let pill = Rect::from_min_size(
            pos2(rect.left() + INSET + x, rect.top() + INSET),
            vec2(w, HEIGHT - 2.0 * INSET),
        );
        let pill_radius = CornerRadius::same(((HEIGHT - 2.0 * INSET) * 0.5) as u8);
        draw::glow(
            painter,
            pill.center(),
            w * 0.6,
            Color32::from_rgba_unmultiplied(10, 132, 255, 40),
        );
        draw::accent_fill(
            painter,
            pill,
            pill_radius,
            if ui.is_enabled() { 1.0 } else { 0.4 },
        );
        draw::sheen(painter, pill, pill_radius);

        let mut left = rect.left() + INSET;
        for (i, (option, w)) in options.iter().zip(&widths).enumerate() {
            let seg = Rect::from_min_size(pos2(left, rect.top()), vec2(*w, HEIGHT));
            let active = i == index;
            let (font, color) = match (ui.is_enabled(), active) {
                (false, _) => (FontId::proportional(14.0), TEXT_FAINT),
                (true, true) => (semibold(14.0), Color32::WHITE),
                (true, false) => (FontId::proportional(14.0), TEXT_DIM),
            };
            let galley = painter.layout_no_wrap((*option).to_owned(), font, color);
            painter.galley(seg.center() - galley.size() * 0.5, galley, color);
            left += w;
        }
    }
    response
}
