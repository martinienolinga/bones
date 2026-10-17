//! Carte d'un préréglage : vignette carrée (squelette animé en boucle, dessin
//! d'aplats) surmontant le nom et un sous-titre. Toucher ouvre la feuille
//! d'application ; appui long sur un préréglage utilisateur, son menu.

use bones_ui::icons::{self, Icon};
use bones_ui::theme;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

use super::library::Entry;
use super::{PresetsState, View, grid};

/// Durée d'appui qui ouvre le menu de la carte.
const LONG_PRESS: f64 = 0.5;

pub fn show(ui: &mut egui::Ui, p: &mut PresetsState, entry: Entry, cell: Vec2, time: f32, fallback: Icon) {
    let (rect, response) = ui.allocate_exact_size(cell, Sense::click());
    let is_user = matches!(entry, Entry::User(_));
    interact(ui, p, entry, is_user, &response);
    if !ui.is_rect_visible(rect) {
        return;
    }

    let press = ui.ctx().animate_bool_with_time(
        response.id.with("press"),
        response.is_pointer_button_down_on(),
        0.12,
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.14);

    let thumb = Rect::from_min_size(rect.min, vec2(rect.width(), rect.width() * 0.78));
    let thumb = scale(thumb, 1.0 - 0.03 * press);
    let radius = CornerRadius::same(12);
    let bg = theme::mix(theme::WIDGET, theme::WIDGET_HOVER, hover.max(press));
    ui.painter().rect_filled(thumb, radius, bg);

    // Vignette animée (squelette) ; repli sur un glyphe tant qu'elle cuit.
    let preset = p.library.preset(entry);
    let name = preset.info.name.clone();
    let subtitle = subtitle(&preset.info);
    let drawn = {
        let painter = ui.painter().with_clip_rect(thumb);
        p.previews.draw(&painter, thumb, preset, time)
    };
    if !drawn {
        let g = Rect::from_center_size(thumb.center(), vec2(26.0, 26.0));
        icons::paint(ui.painter(), g, fallback, theme::TEXT_FAINT);
    }
    // Pastille des préréglages utilisateur (coin haut droit).
    if is_user {
        ui.painter()
            .circle_filled(thumb.right_top() + vec2(-11.0, 11.0), 4.0, theme::ACCENT);
    }
    if press > 0.0 {
        ui.painter()
            .rect_filled(thumb, radius, Color32::from_black_alpha((50.0 * press) as u8));
    }
    if hover > 0.0 {
        ui.painter().rect_stroke(
            thumb,
            radius,
            Stroke::new(1.5, theme::ACCENT.gamma_multiply(hover)),
            StrokeKind::Inside,
        );
    }

    // Nom (blanc) et sous-titre (gris) sous la vignette.
    let painter = ui.painter();
    let name_galley = painter.layout_no_wrap(name, theme::semibold(13.0), theme::TEXT);
    painter.galley(pos2(thumb.left(), thumb.bottom() + 5.0), name_galley, theme::TEXT);
    painter.text(
        pos2(thumb.left(), thumb.bottom() + 22.0),
        Align2::LEFT_TOP,
        subtitle,
        FontId::proportional(11.0),
        theme::TEXT_DIM,
    );
}

/// Toucher (ouvre la feuille) et appui long (menu d'un préréglage utilisateur).
fn interact(ui: &egui::Ui, p: &mut PresetsState, entry: Entry, is_user: bool, response: &egui::Response) {
    let held = response.is_pointer_button_down_on()
        && ui.input(|i| {
            !i.pointer.is_decidedly_dragging()
                && i.pointer
                    .press_start_time()
                    .is_some_and(|t| i.time - t >= LONG_PRESS)
        });
    if is_user && (held || response.secondary_clicked()) && p.menu.is_none() {
        let pos = response.interact_pointer_pos().unwrap_or(response.rect.center());
        grid::open_menu(p, entry, pos);
    }
    if response.clicked() && p.menu.is_none() {
        p.view_slot = Some(View::Apply(entry));
    }
}

/// Sous-titre d'une carte : le groupe, ou à défaut la durée.
fn subtitle(info: &bones_core::presets::PresetInfo) -> String {
    if !info.group.trim().is_empty() {
        info.group.clone()
    } else if info.frames > 0 {
        format!("{:.1} s", info.duration())
    } else {
        "Pose".to_owned()
    }
}

fn scale(rect: Rect, factor: f32) -> Rect {
    Rect::from_center_size(rect.center(), rect.size() * factor)
}
