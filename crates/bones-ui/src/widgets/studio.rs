//! Éléments des « studios » de réglages, à la Brush Studio de Procreate :
//! entrées de navigation (pilule bleue pour la section courante), titres,
//! pastilles de valeur (lecture seule ou à glisser), petits boutons, pas à
//! pas, et boutons Annuler / Valider.

use std::ops::RangeInclusive;

use egui::{
    Align2, Color32, CornerRadius, DragValue, FontId, Rect, Response, Sense, Stroke, StrokeKind, TextStyle,
    Ui, pos2, vec2,
};

use super::{Motion, anim};
use crate::icons::{self, Icon};
use crate::theme::{
    ACCENT, ACCENT_PRESSED, DISABLED, HAIRLINE, ICON_DIM, TEXT, TEXT_LIST, WIDGET, WIDGET_HOVER, mix,
    semibold,
};

/// Entrée de la colonne de navigation : glyphe gris de 20 pt et libellé de
/// 17 pt ; sélectionnée, pilule bleue pleine, glyphe et texte blancs.
pub fn nav_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, 46.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("sel"), selected);
        let painter = ui.painter();
        let radius = CornerRadius::same(23);
        if on > 0.0 {
            painter.rect_filled(rect, radius, ACCENT.gamma_multiply(on));
        } else if motion.hover.max(motion.press) > 0.0 {
            let alpha = (8.0 * motion.hover + 10.0 * motion.press) as u8;
            painter.rect_filled(rect, radius, Color32::from_white_alpha(alpha));
        }
        let glyph = Rect::from_center_size(pos2(rect.left() + 28.0, rect.center().y), vec2(20.0, 20.0));
        icons::paint(painter, glyph, icon, mix(ICON_DIM, Color32::WHITE, on));
        painter.text(
            pos2(glyph.right() + 14.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(17.0),
            mix(TEXT_LIST, Color32::WHITE, on),
        );
    }
    response
}

/// Titre de section du studio (24 pt semi-gras).
pub fn studio_title(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).font(semibold(24.0)).color(TEXT));
}

fn chip_rect(
    ui: &mut Ui,
    text: &str,
    min_width: f32,
    sense: Sense,
) -> (Rect, Response, std::sync::Arc<egui::Galley>) {
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(15.0), TEXT);
    let size = vec2((galley.size().x + 24.0).max(min_width), 30.0);
    let (rect, response) = ui.allocate_exact_size(size, sense);
    (rect, response, galley)
}

/// Pastille de valeur en lecture seule (« 17 % », « None »).
pub fn value_chip(ui: &mut Ui, text: &str) -> Response {
    let (rect, response, galley) = chip_rect(ui, text, 44.0, Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, CornerRadius::same(15), WIDGET);
        let color = if ui.is_enabled() { TEXT } else { DISABLED };
        ui.painter()
            .galley(rect.center() - galley.size() * 0.5, galley, color);
    }
    response
}

/// Pastille de valeur modifiable : glisser pour régler, toucher pour saisir.
pub fn drag_chip(
    ui: &mut Ui,
    value: &mut f32,
    range: RangeInclusive<f32>,
    speed: f64,
    decimals: usize,
    suffix: &str,
    width: f32,
) -> Response {
    ui.scope(|ui| {
        let style = ui.style_mut();
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(15.0));
        style.spacing.interact_size.y = 30.0;
        style.spacing.button_padding = vec2(10.0, 4.0);
        let w = &mut style.visuals.widgets;
        for v in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            v.corner_radius = CornerRadius::same(15);
            v.expansion = 0.0;
        }
        ui.add_sized(
            [width, 30.0],
            DragValue::new(value)
                .range(range)
                .speed(speed)
                .fixed_decimals(decimals)
                .suffix(suffix),
        )
    })
    .inner
}

/// Petit bouton pilule de 32 pt (alignement X / Y / Z, navigation) ; actif :
/// fond bleu.
pub fn chip_button(ui: &mut Ui, label: &str, active: bool) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), semibold(14.0), TEXT);
    let size = vec2((galley.size().x + 26.0).max(44.0), 34.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let on = anim(ui, response.id.with("active"), active);
        let idle = mix(WIDGET, WIDGET_HOVER, motion.hover.max(motion.press));
        let fill = mix(idle, ACCENT, on);
        ui.painter().rect_filled(rect, CornerRadius::same(17), fill);
        let color = if ui.is_enabled() { TEXT } else { DISABLED };
        ui.painter()
            .galley(rect.center() - galley.size() * 0.5, galley, color);
    }
    response
}

/// Pas à pas « − n + » ; `changed()` quand la valeur change.
pub fn stepper(ui: &mut Ui, value: &mut usize, range: RangeInclusive<usize>) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(112.0, 34.0), Sense::click());
    let third = rect.width() / 3.0;
    if response.clicked()
        && let Some(p) = response.interact_pointer_pos()
    {
        let old = *value;
        if p.x < rect.left() + third {
            *value = value.saturating_sub(1).max(*range.start());
        } else if p.x > rect.right() - third {
            *value = (*value + 1).min(*range.end());
        }
        if *value != old {
            response.mark_changed();
        }
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(17), WIDGET);
        let color = if ui.is_enabled() { TEXT } else { DISABLED };
        let y = rect.center().y;
        for (x, sign) in [
            (rect.left() + third * 0.5, "−"),
            (rect.right() - third * 0.5, "+"),
        ] {
            painter.text(
                pos2(x, y),
                Align2::CENTER_CENTER,
                sign,
                FontId::proportional(20.0),
                color,
            );
        }
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            value.to_string(),
            semibold(15.0),
            color,
        );
    }
    response
}

/// Bouton « Cancel » : pilule cerclée d'un filet gris, texte de 17 pt.
pub fn cancel_button(ui: &mut Ui, label: &str) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(17.0), TEXT);
    let size = vec2(galley.size().x + 32.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let painter = ui.painter();
        let radius = CornerRadius::same(20);
        if motion.press > 0.0 {
            painter.rect_filled(rect, radius, WIDGET.gamma_multiply(motion.press));
        }
        painter.rect_stroke(rect, radius, Stroke::new(1.0, HAIRLINE), StrokeKind::Inside);
        painter.galley(rect.center() - galley.size() * 0.5, galley, TEXT);
    }
    response
}

/// Bouton de validation : disque bleu de 40 pt et coche blanche.
pub fn confirm_button(ui: &mut Ui, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let motion = Motion::of(ui, &response);
        let fill = mix(ACCENT, ACCENT_PRESSED, motion.hover.max(motion.press));
        ui.painter().circle_filled(rect.center(), 20.0, fill);
        let glyph = Rect::from_center_size(rect.center(), vec2(22.0, 22.0));
        icons::paint(ui.painter(), glyph, Icon::Check, Color32::WHITE);
    }
    response.on_hover_text(hint)
}
