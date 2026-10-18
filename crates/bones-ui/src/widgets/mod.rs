//! Widgets tactiles à la Procreate : boutons et outils à glyphes
//! monochromes, puits ronds de la barre, contrôle segmenté, curseurs,
//! interrupteurs, listes groupées, studio de réglages, frise de clés et
//! commandes de la galerie (capsule, bouton rond, pastille de sélection).
//!
//! Toutes les transitions (survol, pression, activation) sont animées par
//! egui, qui ne redemande un rendu que pendant l'animation.

mod bar;
mod buttons;
mod capsule;
mod group;
mod rows;
mod segmented;
mod slider;
mod slider_row;
mod studio;
mod switch;
mod tile;
mod timeline;
mod tree;

pub use bar::{bar_tab, bar_tool, square_button, tab, well_button};
pub use buttons::{pill_button, separator_dot, text_button, tool_button, tool_cell};
pub use capsule::{CapsuleItem, CapsuleResponse, CapsuleTone, capsule, check_badge, round_button};
pub use group::{Group, group};
pub use rows::{list_row, note, section_title};
pub use segmented::segmented;
pub use slider::{vertical_slider, vertical_slider_value, vertical_slider_with_icon};
pub use slider_row::{slider_row, slider_row_with};
pub use studio::{
    cancel_button, chip_button, confirm_button, drag_chip, nav_item, stepper, studio_title, value_chip,
};
pub use switch::{switch, switch_row};
pub use tile::tile;
pub use timeline::key_strip;
pub use tree::{RowResponse, RowSpec, outliner_row};

use egui::{Color32, Id, Response, Ui};

use crate::theme::{ACCENT, DISABLED, ICON, mix};

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

/// Couleur d'un glyphe : gris clair au repos, bleu d'accent selon `on`
/// (0 → 1), gris foncé si le widget est désactivé.
fn glyph_color(ui: &Ui, on: f32) -> Color32 {
    if ui.is_enabled() {
        mix(ICON, ACCENT, on)
    } else {
        DISABLED
    }
}
