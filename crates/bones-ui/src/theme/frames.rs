//! Cadres : blocs posés sur la vue (frise, bulles) et fenêtres
//! contextuelles. Aplats, coins arrondis, ombre large et légère sous les
//! fenêtres seulement.

use egui::{Color32, CornerRadius, Frame, Margin, Shadow, Stroke};

use super::{BAR, PANEL, PANEL_BORDER};

/// Ombre large et légère des fenêtres contextuelles.
pub fn soft_shadow() -> Shadow {
    Shadow {
        offset: [0, 12],
        blur: 40,
        spread: 0,
        color: Color32::from_black_alpha(64),
    }
}

/// Ombre très courte des petites bulles flottantes et des pouces.
pub fn small_shadow() -> Shadow {
    Shadow {
        offset: [0, 1],
        blur: 3,
        spread: 0,
        color: Color32::from_black_alpha(60),
    }
}

/// Fenêtre contextuelle (galerie, actions, réglages…).
pub fn panel_frame() -> Frame {
    Frame::new()
        .fill(PANEL)
        .corner_radius(CornerRadius::same(20))
        .inner_margin(Margin::same(20))
        .shadow(soft_shadow())
}

/// Bloc flottant (frise, messages).
pub fn bar_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(6))
}

/// Petite carte (boutons du guide de rotation).
pub fn card_frame() -> Frame {
    Frame::new()
        .fill(BAR)
        .stroke(Stroke::new(1.0, PANEL_BORDER))
        .corner_radius(CornerRadius::same(14))
        .inner_margin(Margin::same(4))
}
