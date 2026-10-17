//! Réglages du style egui : typographie, espacements tactiles, états des
//! widgets natifs (boutons, cases, champs) accordés au reste du thème.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

use super::*;

/// Applique le thème (police comprise) à tout le contexte egui.
pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, semibold(19.0)),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();
        style.animation_time = 0.16;

        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(8.0, 8.0);
        spacing.button_padding = egui::vec2(14.0, 9.0);
        spacing.interact_size = egui::vec2(44.0, 40.0);
        spacing.slider_width = 200.0;
        spacing.icon_width = 22.0;
        spacing.icon_width_inner = 12.0;
        spacing.icon_spacing = 8.0;
        spacing.window_margin = Margin::same(16);
        spacing.menu_margin = Margin::same(12);
        spacing.scroll.bar_width = 4.0;
        spacing.scroll.floating = true;

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(TEXT);
        v.hyperlink_color = ACCENT;
        v.selection.bg_fill = ACCENT;
        v.selection.stroke = Stroke::new(1.5, Color32::WHITE);
        v.window_fill = PANEL;
        v.panel_fill = PANEL;
        v.window_stroke = Stroke::new(1.0, PANEL_BORDER);
        v.window_corner_radius = CornerRadius::same(18);
        v.menu_corner_radius = CornerRadius::same(14);
        v.window_shadow = soft_shadow();
        v.popup_shadow = soft_shadow();
        v.extreme_bg_color = Color32::from_rgb(0x17, 0x17, 0x1a);
        v.faint_bg_color = BAR;
        v.error_fg_color = DANGER;
        v.warn_fg_color = Color32::from_rgb(0xff, 0x9f, 0x0a);

        let radius = CornerRadius::same(10);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = PANEL;
        w.noninteractive.weak_bg_fill = PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_white_alpha(14));
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
        w.noninteractive.corner_radius = radius;
        for (visuals, fill, fg) in [
            (&mut w.inactive, WIDGET, TEXT),
            (&mut w.hovered, WIDGET_HOVER, TEXT),
            (&mut w.active, ACCENT, Color32::WHITE),
            (&mut w.open, WIDGET_HOVER, TEXT),
        ] {
            visuals.bg_fill = fill;
            visuals.weak_bg_fill = fill;
            visuals.bg_stroke = Stroke::NONE;
            visuals.corner_radius = radius;
            visuals.fg_stroke = Stroke::new(1.5, fg);
            visuals.expansion = 0.0;
        }
        w.hovered.expansion = 1.0;
    });
}
