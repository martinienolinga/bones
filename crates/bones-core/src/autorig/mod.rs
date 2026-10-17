//! Rigging assisté inspiré d'Auto-Rig Pro (mode « Smart ») : quelques
//! marqueurs placés sur le modèle vu de face suffisent à générer un squelette
//! humanoïde complet (colonne, cou, tête, bras, mains, doigts, jambes, pieds)
//! et ses membres IK/FK.
//!
//! Le personnage regarde vers +Z, Y vers le haut ; le côté gauche (`.l`) est
//! en +X. Le côté droit est obtenu par symétrie autour du plan x = racine.x.
//!
//! Étapes : marqueurs → [`Layout`] (positions des articulations, proportions
//! génériques) → ajustement facultatif au maillage ([`fit_layout`]) → os et
//! membres ([`build`]).

mod fit;
mod generate;
mod layout;
mod limb;
mod markers;
#[cfg(test)]
mod tests;

pub use fit::fit_layout;
pub use generate::{AutoRigOptions, build, generate, generate_fitted};
pub use layout::{FINGER_NAMES, Layout};
pub use limb::{Limb, LimbMode};
pub use markers::{MarkerKind, Markers};
