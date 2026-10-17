//! Rigging assisté inspiré d'Auto-Rig Pro (mode « Smart ») : quelques
//! marqueurs placés sur le modèle vu de face suffisent à générer un squelette
//! humanoïde complet (colonne, cou, tête, bras, mains, doigts, jambes, pieds)
//! et ses membres IK/FK.
//!
//! Le personnage regarde vers +Z, Y vers le haut ; le côté gauche (`.l`) est
//! en +X. Le côté droit est obtenu par symétrie autour du plan x = racine.x.

mod generate;
mod limb;
mod markers;
#[cfg(test)]
mod tests;

pub use generate::{AutoRigOptions, generate};
pub use limb::{Limb, LimbMode};
pub use markers::{MarkerKind, Markers};
