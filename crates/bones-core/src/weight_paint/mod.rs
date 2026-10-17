//! Peinture de poids (mode « Weight Paint » de Blender) : pinceaux, modes de
//! mélange, atténuation, symétrie, normalisation automatique et outils du
//! menu « Weights ».
//!
//! Le moteur ne dépend ni du GPU ni de l'interface : l'éditeur
//! ([`crate::Editor`]) lui transmet les gestes, et le rendu relit les poids
//! quantifiés du [`crate::Skin`] quand sa révision change.

mod settings;
mod state;

pub use settings::{
    Blend, BrushSettings, Falloff, GradientKind, PaintTool, WeightPaintSettings, WeightsTool,
};
pub(crate) use state::PaintState;
