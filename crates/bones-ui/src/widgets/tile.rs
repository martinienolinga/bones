//! Tuile de grille à la Procreate : une vignette carrée arrondie surmontant
//! un libellé, sélectionnable. Sélectionnée, un liseré bleu cercle la
//! vignette et le libellé passe au bleu d'accent. Le contenu de la vignette
//! (aperçu) est dessiné par l'appelant, dans le style plat du thème.

use egui::{CornerRadius, FontId, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

use super::{Motion, anim};
use crate::theme::{ACCENT, TEXT_DIM, WIDGET, WIDGET_HOVER, mix, semibold};

/// Hauteur réservée au libellé sous la vignette.
const LABEL_H: f32 = 18.0;

/// Tuile `size` (vignette + libellé) ; `paint` dessine l'aperçu dans le
/// rectangle fourni (déjà margé). Réponse `clicked()` au toucher.
pub fn tile(
    ui: &mut Ui,
    size: Vec2,
    label: &str,
    active: bool,
    paint: impl FnOnce(&Painter, Rect),
) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let painter = ui.painter();
        let thumb = Rect::from_min_size(rect.min, vec2(rect.width(), rect.height() - LABEL_H));
        let radius = CornerRadius::same(12);
        painter.rect_filled(
            thumb,
            radius,
            mix(WIDGET, WIDGET_HOVER, motion.hover.max(motion.press)),
        );
        paint(painter, thumb.shrink(8.0));
        if on > 0.0 {
            painter.rect_stroke(
                thumb,
                radius,
                Stroke::new(2.0, ACCENT.gamma_multiply(on)),
                StrokeKind::Inside,
            );
        }
        let color = mix(TEXT_DIM, ACCENT, on);
        let font = if active {
            semibold(12.0)
        } else {
            FontId::proportional(12.0)
        };
        let galley = painter.layout_no_wrap(label.to_owned(), font, color);
        let pos = pos2(rect.center().x - galley.size().x * 0.5, thumb.bottom() + 3.0);
        painter.galley(pos, galley, color);
    }
    response.on_hover_text(label)
}
