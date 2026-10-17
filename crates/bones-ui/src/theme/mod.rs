//! Thème sombre et plat, dans l'esprit de Procreate et Nomad Sculpt : blocs
//! très sombres aux coins généreux, liseré d'un pixel, ombres courtes, un
//! accent bleu iOS en aplat ; aucun dégradé.
//!
//! - couleurs (ce module) ;
//! - [`fonts`] : police Inter embarquée ;
//! - [`style`] : réglages egui (espacements, états des widgets) ;
//! - [`frames`] : cadres des blocs, cartes et fenêtres contextuelles.

mod fonts;
mod frames;
mod style;

use egui::Color32;

pub use fonts::{install_fonts, semibold};
pub use frames::{bar_frame, card_frame, panel_frame, small_shadow, soft_shadow};
pub use style::apply;

/// Couleur de fond de la vue 3D (sRGB #262628), en espace d'affichage : la
/// surface wgpu est en `Bgra8Unorm` (non sRGB), la valeur est écrite telle
/// quelle.
pub const BACKGROUND: [f64; 3] = [0.149, 0.149, 0.157];

/// Blocs posés sur la vue (barre du haut, cartes latérales).
pub const BAR: Color32 = Color32::from_rgba_premultiplied(22, 22, 25, 244);
/// Fenêtres contextuelles.
pub const PANEL: Color32 = Color32::from_rgb(0x21, 0x21, 0x25);
/// Liseré de lumière intérieur des blocs (blanc 7 %).
pub const PANEL_BORDER: Color32 = Color32::from_rgba_premultiplied(18, 18, 18, 18);
/// Fond des boutons au repos et des pistes.
pub const WIDGET: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x34);
/// Fond survolé / pressé.
pub const WIDGET_HOVER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x41);
/// Pouce des curseurs (gris perle).
pub const THUMB: Color32 = Color32::from_rgb(0xe5, 0xe5, 0xea);
/// Accent : bleu iOS de Procreate.
pub const ACCENT: Color32 = Color32::from_rgb(0x0a, 0x84, 0xff);
/// Accent survolé / pressé (un ton plus sombre).
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x00, 0x6e, 0xdb);
/// Accent atténué (fonds de sélection).
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(4, 46, 89, 89);
pub const TEXT: Color32 = Color32::from_rgb(0xf5, 0xf5, 0xf7);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8e, 0x8e, 0x93);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x63, 0x63, 0x66);
pub const DANGER: Color32 = Color32::from_rgb(0xff, 0x45, 0x3a);
pub const SUCCESS: Color32 = Color32::from_rgb(0x30, 0xd1, 0x58);

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

/// Mélange linéaire de deux couleurs (prémultipliées).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), l(a.a(), b.a()))
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
    fn mix_interpolates() {
        assert_eq!(mix(Color32::BLACK, Color32::WHITE, 0.0), Color32::BLACK);
        assert_eq!(mix(Color32::BLACK, Color32::WHITE, 1.0), Color32::WHITE);
        assert_eq!(mix(Color32::BLACK, Color32::WHITE, 0.5).r(), 128);
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
