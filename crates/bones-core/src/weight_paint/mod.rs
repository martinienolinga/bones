//! Peinture de poids (mode « Weight Paint » de Blender) : pinceaux, modes de
//! mélange, atténuation, symétrie, normalisation automatique et outils du
//! menu « Weights ».
//!
//! Le moteur ne dépend ni du GPU ni de l'interface : l'éditeur
//! ([`crate::Editor`]) lui transmet les gestes, et le rendu relit les poids
//! quantifiés du [`crate::Skin`] quand sa révision change.
//!
//! Organisation : [`settings`] (réglages sérialisés), [`influence`] (copie de
//! travail flottante, normalisation, limite à quatre os), [`caches`] (sommets
//! symétriques et voisinage, par révision de maillage), [`grid`] (grille
//! écran), [`project`] (projection des sommets déformés), [`stroke`] +
//! [`gradient`] (trait de pinceau) et [`weights_tool`] (menu « Weights »).

mod caches;
mod gradient;
mod grid;
mod influence;
mod project;
mod settings;
mod state;
mod stroke;
#[cfg(test)]
mod tests;
mod weights_tool;

pub use settings::{
    Blend, BrushSettings, Falloff, GradientKind, PaintTool, WeightPaintSettings, WeightsTool,
};

pub(crate) use caches::MeshCaches;
pub(crate) use project::project_vertices;
pub(crate) use state::PaintState;
pub(crate) use stroke::Stroke;
pub(crate) use weights_tool::run as run_weights_tool;
