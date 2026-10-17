//! Cadres : blocs posés sur la vue (barre, cartes) et fenêtres
//! contextuelles. Aplats, coins généreux, liseré d'un pixel, ombre courte.

use egui::{Color32, CornerRadius, Frame, Margin, Shadow, Stroke};

use super::{BAR, PANEL, PANEL_BORDER};

/// Ombre portée nette des blocs flottants.
pub fn soft_shadow() -> Shadow {
    Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(80),
    }
}

/// Ombre très courte des petites bulles flottantes (valeur d'un curseur).
pub fn small_shadow() -> Shadow {
    Shadow {
        offset: [0, 2],
        blur: 4,
        spread: 0,
        color: Color32::from_black_alpha(70),
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
