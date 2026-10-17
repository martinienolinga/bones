//! Contrôle segmenté en pilule (choix du mode, de l'interpolation…).

use egui::{Color32, CornerRadius, FontId, Rect, Response, Sense, Ui, pos2, vec2};

use crate::theme::{ACCENT, TEXT, TEXT_DIM, WIDGET};

const HEIGHT: f32 = 32.0;
const PAD: f32 = 14.0;

/// Pilule à plusieurs segments ; le segment `*selected` est rempli
/// d'accent. La réponse est marquée `changed()` quand la sélection change.
pub fn segmented(ui: &mut Ui, options: &[&str], selected: &mut usize) -> Response {
    let font = FontId::proportional(14.0);
    let galleys: Vec<_> = options
        .iter()
        .map(|o| ui.painter().layout_no_wrap((*o).to_owned(), font.clone(), TEXT))
        .collect();
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 2.0 * PAD).collect();
    let total = widths.iter().sum::<f32>().max(HEIGHT);
    let (rect, mut response) = ui.allocate_exact_size(vec2(total, HEIGHT), Sense::click());

    // Segment sous le doigt au moment du clic.
    if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let mut x = rect.left();
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
        painter.rect_filled(rect, radius, WIDGET);
        let mut x = rect.left();
        for (i, (galley, w)) in galleys.into_iter().zip(&widths).enumerate() {
            let seg = Rect::from_min_size(pos2(x, rect.top()), vec2(*w, HEIGHT));
            let active = i == *selected;
            if active {
                painter.rect_filled(seg.shrink(2.0), radius, ACCENT);
            }
            let color = match (ui.is_enabled(), active) {
                (false, _) => TEXT_DIM.gamma_multiply(0.5),
                (true, true) => Color32::WHITE,
                (true, false) => TEXT_DIM,
            };
            painter.galley(seg.center() - galley.size() * 0.5, galley, color);
            x += w;
        }
    }
    response
}
