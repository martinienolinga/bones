//! Cadres : blocs posés sur la vue (barre, cartes) et fenêtres
//! contextuelles. Coins généreux, liseré de lumière intérieur, ombre longue.

use egui::{Color32, CornerRadius, Frame, Margin, Shadow, Stroke};

use super::{BAR, PANEL, PANEL_BORDER};

/// Ombre douce et longue des blocs flottants.
pub fn soft_shadow() -> Shadow {
    Shadow {
        offset: [0, 10],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(120),
    }
}

/// Fenêtre contextuelle (galerie, actions, réglages…).
pub fn panel_frame() -> Frame {
    Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, PANEL_BORDER))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::same(16))
        .shadow(soft_shadow())
}

/// Bloc de la barre supérieure et de la frise.
pub fn bar_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .stroke(Stroke::new(1.0, PANEL_BORDER))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(6))
        .shadow(soft_shadow())
}

/// Carte latérale (curseurs, boîte à outils).
pub fn card_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .stroke(Stroke::new(1.0, PANEL_BORDER))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(6))
        .shadow(soft_shadow())
}
