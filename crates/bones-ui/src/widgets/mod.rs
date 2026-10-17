//! Widgets tactiles animés : boutons d'outils (Procreate), cellules de boîte
//! à outils (Nomad Sculpt), contrôle segmenté à pilule glissante, curseurs,
//! interrupteurs iOS, lignes de liste et frise de clés.
//!
//! Toutes les transitions (survol, pression, activation) sont animées par
//! egui, qui ne redemande un rendu que pendant l'animation.

mod buttons;
mod rows;
mod segmented;
mod slider;
mod switch;
mod timeline;

pub use buttons::{pill_button, separator_dot, text_button, tool_button, tool_cell};
pub use rows::{list_row, section_title};
pub use segmented::segmented;
pub use slider::{slider_row, vertical_slider, vertical_slider_with_icon};
pub use switch::{switch, switch_row};
pub use timeline::key_strip;

use egui::{Color32, Id, Response, Ui};

/// Durée des transitions d'état, en secondes.
const EASE: f32 = 0.14;

/// Valeur animée (0 → 1) d'un état booléen.
fn anim(ui: &Ui, id: Id, on: bool) -> f32 {
    ui.ctx().animate_bool_with_time(id, on, EASE)
}

/// États animés d'un widget : survol, pression.
struct Motion {
    hover: f32,
    press: f32,
}

impl Motion {
    fn of(ui: &Ui, response: &Response) -> Self {
        Self {
            hover: anim(ui, response.id.with("hover"), response.hovered()),
            press: anim(
                ui,
                response.id.with("press"),
                response.is_pointer_button_down_on(),
            ),
        }
    }
}

/// Teinte d'icône selon que le widget est actif ou désactivé.
fn icon_tint(ui: &Ui) -> Color32 {
    if ui.is_enabled() {
        Color32::WHITE
    } else {
        Color32::from_white_alpha(80)
    }
}
