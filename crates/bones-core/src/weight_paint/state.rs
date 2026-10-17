//! État de la peinture de poids gardé par l'éditeur : pression du crayon,
//! trait en cours et caches dérivés du maillage.

use super::caches::MeshCaches;
use super::stroke::Stroke;

/// Caches (symétrie, voisinage) et trait en cours.
pub(crate) struct PaintState {
    /// Pression du crayon (0 à 1 ; 1 au doigt ou à la souris).
    pub pressure: f32,
    /// Trait de pinceau en cours (entre `pointer_down` et `pointer_up`).
    pub stroke: Option<Stroke>,
    /// Caches du maillage (symétrie, voisinage), reconstruits par révision.
    pub caches: Option<MeshCaches>,
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            pressure: 1.0,
            stroke: None,
            caches: None,
        }
    }
}
