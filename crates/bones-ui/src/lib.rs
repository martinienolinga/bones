//! Kit d'interface de PaintSharp Bones, construit sur egui.
//!
//! Calqué sur Procreate : barre pleine en haut, bandeaux collés aux bords,
//! fenêtres #2C2C2E aux coins de 20 pt, listes groupées et studios de
//! réglages ; style plat (aucun dégradé), glyphes monochromes teintés en
//! bleu d'accent quand ils sont actifs, police Inter, transitions animées et
//! cibles tactiles d'au moins 44 pt.
//!
//! - [`theme`] : couleurs, police, style egui, cadres de panneaux ;
//! - [`icons`] : glyphes SVG monochromes rastérisés dans un atlas ;
//! - [`widgets`] : boutons, puits et outils de barre, contrôle segmenté,
//!   curseurs, interrupteurs, listes groupées, studio, frise de clés.
//!
//! Appeler [`install`] une fois au démarrage.

pub mod icons;
pub mod theme;
pub mod widgets;

pub use icons::Icon;
pub use widgets::{
    key_strip, list_row, pill_button, section_title, segmented, separator_dot, slider_row, switch,
    switch_row, text_button, tool_button, tool_cell, vertical_slider, vertical_slider_with_icon,
};

/// Installe le thème, la police et prépare l'atlas d'icônes.
pub fn install(ctx: &egui::Context) {
    theme::apply(ctx);
    icons::install(ctx);
}

#[cfg(test)]
mod tests;
