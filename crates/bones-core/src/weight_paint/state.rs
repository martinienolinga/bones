//! État de la peinture de poids gardé par l'éditeur : caches et trait en cours.

/// Caches (copie de travail des poids, symétrie, voisinage) et trait en cours.
#[derive(Default)]
pub(crate) struct PaintState {
    /// Pression du crayon (0 à 1 ; 1 au doigt ou à la souris).
    pub pressure: f32,
}
