//! Kit d'interface de PaintSharp Bones, construit sur egui.
//!
//! Inspiré de Procreate et Nomad Sculpt : blocs sombres posés sur la vue 3D,
//! icônes SVG en couleur par famille d'outils, police Inter, un accent bleu
//! qui glisse vers l'indigo, transitions animées et cibles tactiles d'au
//! moins 44 pt.
//!
//! - [`theme`] : couleurs, police, style egui, cadres de panneaux ;
//! - [`icons`] : icônes SVG rastérisées dans un atlas ;
//! - [`draw`] : dégradés, lueurs et reflets partagés ;
//! - [`widgets`] : boutons d'outils, cellules, contrôle segmenté, curseurs,
//!   interrupteurs, lignes de liste, frise de clés.
//!
//! Appeler [`install`] une fois au démarrage.

pub mod draw;
pub mod icons;
pub mod theme;
pub mod widgets;

pub use icons::Icon;
pub use widgets::{
    key_strip, list_row, pill_button, section_title, segmented, separator_dot, slider_row, switch,
    switch_row, text_button, tool_button, tool_cell, vertical_slider, vertical_slider_with_icon,
};

/// Installe le thème, la police et prépare les textures (icônes, dégradés).
pub fn install(ctx: &egui::Context) {
    theme::apply(ctx);
    draw::install(ctx);
    icons::install(ctx);
}

#[cfg(test)]
mod tests;
