//! Kit d'interface de PaintSharp Bones, construit sur egui.
//!
//! Inspiré de Procreate et Nomad Sculpt : blocs sombres opaques posés sur
//! la vue 3D, icônes vectorielles au trait fin, un unique accent bleu et des
//! cibles tactiles d'au moins 40 pt.
//!
//! - [`theme`] : couleurs, style egui, cadres de panneaux ;
//! - [`icons`] : jeu d'icônes dessinées au `Painter` ;
//! - [`widgets`] : boutons d'outils, cellules de boîte à outils, contrôle
//!   segmenté, curseurs verticaux et frise de clés.

pub mod icons;
pub mod theme;
pub mod widgets;

pub use icons::Icon;
pub use widgets::{
    key_strip, segmented, separator_dot, text_button, tool_button, tool_cell, vertical_slider,
    vertical_slider_with_icon,
};

#[cfg(test)]
mod tests;
