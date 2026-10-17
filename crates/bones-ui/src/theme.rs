//! Thème sombre et minimaliste, relevé sur des captures de Procreate et
//! Nomad Sculpt : blocs opaques très sombres aux coins arrondis posés sur la
//! vue 3D, un seul accent bleu iOS, cibles tactiles généreuses.

use egui::{Color32, CornerRadius, FontFamily, FontId, Frame, Margin, Shadow, Stroke, TextStyle};

/// Couleur de fond de la vue 3D (sRGB #262628), en espace d'affichage : la
/// surface wgpu est en `Bgra8Unorm` (non sRGB), la valeur est écrite telle
/// quelle.
pub const BACKGROUND: [f64; 3] = [0.149, 0.149, 0.157];

/// Blocs de la barre supérieure et cartes latérales (Nomad Sculpt), opaques.
pub const BAR: Color32 = Color32::from_rgb(0x18, 0x18, 0x18);
/// Fond des popovers et panneaux flottants, opaque.
pub const PANEL: Color32 = Color32::from_rgb(0x2e, 0x2b, 0x2c);
/// Liseré discret des panneaux.
pub const PANEL_BORDER: Color32 = Color32::from_rgba_unmultiplied_const(255, 255, 255, 20);
/// Fond des boutons au repos et des pistes de curseurs.
pub const WIDGET: Color32 = Color32::from_rgb(0x3d, 0x3b, 0x3b);
/// Pouce des curseurs verticaux (gris clair, façon Procreate).
pub const THUMB: Color32 = Color32::from_rgb(0xa5, 0xa5, 0xa5);
/// Accent unique (outil actif, sélection) : bleu iOS de Procreate.
pub const ACCENT: Color32 = Color32::from_rgb(0x0a, 0x84, 0xff);
/// Accent atténué (cellule d'outil active, survol).
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_unmultiplied_const(0x0a, 0x84, 0xff, 90);
pub const TEXT: Color32 = Color32::WHITE;
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x73, 0x73, 0x73);
pub const DANGER: Color32 = Color32::from_rgb(255, 82, 82);

// Couleurs de rigging (convention proche d'Auto-Rig Pro / Blender).

/// Os central (colonne, tête…), jaune chaud.
pub const BONE_CENTER: Color32 = Color32::from_rgb(232, 196, 92);
/// Os du côté gauche (`.l`), bleu.
pub const BONE_LEFT: Color32 = Color32::from_rgb(84, 156, 255);
/// Os du côté droit (`.r`), rouge rosé.
pub const BONE_RIGHT: Color32 = Color32::from_rgb(255, 96, 128);
/// Os sélectionné, orange comme dans Blender.
pub const BONE_SELECTED: Color32 = Color32::from_rgb(255, 160, 40);
/// Os épinglé (point d'appui), vert.
pub const BONE_PINNED: Color32 = Color32::from_rgb(92, 214, 120);
/// Poignée de cible IK.
pub const IK_HANDLE: Color32 = Color32::from_rgb(255, 214, 64);
/// Poignée de pôle IK (coude, genou).
pub const POLE_HANDLE: Color32 = Color32::from_rgb(196, 120, 255);
/// Trajectoire de l'os sélectionné.
pub const TRAJECTORY: Color32 = Color32::from_rgb(120, 220, 255);
/// Poses fantômes (onion skin), translucides.
pub const GHOST: Color32 = Color32::from_rgba_unmultiplied_const(170, 190, 255, 90);
/// Marqueurs de l'auto-rig.
pub const MARKER: Color32 = Color32::from_rgb(80, 240, 200);

/// Couleur `Color32` (sRGB, alpha non prémultiplié) convertie en RGBA
/// linéaire pour le GPU.
///
/// Utilise un gamma de 2,2, l'inverse exact de la conversion faite en fin de
/// shader : une couleur de l'interface et la même couleur dessinée en 3D
/// sont donc identiques à l'écran.
pub fn to_linear(color: Color32) -> [f32; 4] {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    let lin = |c: u8| (c as f32 / 255.0).powf(2.2);
    [lin(r), lin(g), lin(b), a as f32 / 255.0]
}

/// Applique le thème à tout le contexte egui.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
        ]
        .into();

        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(8.0, 8.0);
        spacing.button_padding = egui::vec2(12.0, 8.0);
        spacing.interact_size = egui::vec2(40.0, 40.0);
        spacing.slider_width = 180.0;
        spacing.icon_width = 22.0;
        spacing.icon_width_inner = 12.0;
        spacing.window_margin = Margin::same(12);
        spacing.menu_margin = Margin::same(10);

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(TEXT);
        v.hyperlink_color = ACCENT;
        v.selection.bg_fill = ACCENT;
        v.selection.stroke = Stroke::new(1.5, Color32::WHITE);
        v.window_fill = PANEL;
        v.panel_fill = PANEL;
        v.window_stroke = Stroke::new(1.0, PANEL_BORDER);
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(12);
        v.window_shadow = soft_shadow();
        v.popup_shadow = soft_shadow();
        v.extreme_bg_color = WIDGET;
        v.faint_bg_color = BAR;
        v.error_fg_color = DANGER;

        let radius = CornerRadius::same(8);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0, PANEL_BORDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
        for (visuals, fill) in [
            (&mut w.inactive, WIDGET),
            (&mut w.hovered, Color32::from_rgb(0x4a, 0x48, 0x48)),
            (&mut w.active, ACCENT),
            (&mut w.open, Color32::from_rgb(0x4a, 0x48, 0x48)),
        ] {
            visuals.bg_fill = fill;
            visuals.weak_bg_fill = fill;
            visuals.bg_stroke = Stroke::NONE;
            visuals.corner_radius = radius;
            visuals.fg_stroke = Stroke::new(1.5, TEXT);
            visuals.expansion = 0.0;
        }
        w.noninteractive.corner_radius = radius;
    });
}

fn soft_shadow() -> Shadow {
    Shadow {
        offset: [0, 4],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(90),
    }
}

/// Cadre des popovers et panneaux flottants (12 pt, ombre douce).
pub fn panel_frame() -> Frame {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, PANEL_BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(10))
        .shadow(soft_shadow())
}

/// Cadre des blocs de la barre supérieure (Procreate / Nomad Sculpt).
pub fn bar_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(6))
}

/// Carte latérale empilable façon Nomad Sculpt (curseurs, boîte à outils).
pub fn card_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_conversion_bounds() {
        assert_eq!(to_linear(Color32::WHITE), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(to_linear(Color32::BLACK), [0.0, 0.0, 0.0, 1.0]);
        let [r, g, b, a] = to_linear(GHOST);
        assert!(r > 0.0 && r < 1.0 && g < 1.0 && b <= 1.0);
        assert!((a - 90.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn apply_sets_touch_friendly_style() {
        let ctx = egui::Context::default();
        apply(&ctx);
        let style = ctx.global_style();
        assert!(style.spacing.interact_size.y >= 40.0);
        assert_eq!(style.visuals.selection.bg_fill, ACCENT);
    }
}
