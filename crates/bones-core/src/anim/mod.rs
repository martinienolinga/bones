//! Animation par poses-clés, à la manière de Cascadeur : une clé enregistre
//! la pose complète du personnage à une image donnée, et chaque intervalle
//! entre deux clés a son propre mode d'interpolation.

mod clip;
mod interpolate;
pub mod physics;
#[cfg(test)]
mod tests;

pub use clip::{Clip, Interpolation, Key};
