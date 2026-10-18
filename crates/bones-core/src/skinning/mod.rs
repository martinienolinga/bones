//! Skinning : poids des sommets (format compact), calcul automatique
//! (enveloppes ou diffusion de chaleur) et déformation.

mod closest;
mod deform;
pub mod envelope;
mod face;
pub mod heat;
mod sparse;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_visibility;
mod transfer;
mod visibility;
mod weights;

use glam::Vec3;
use serde::{Deserialize, Serialize};

pub use envelope::distance_to_segment;
pub use heat::heat_weights;
pub use weights::{MAX_INFLUENCES, Skin, VertexWeights};

use crate::{BoneId, Mesh, Skeleton, head_tail};

/// Segments tête–queue (pose de repos) des os déformants, et leur
/// identifiant : les os de contrôle (`deform == false`) ne reçoivent aucun poids.
pub(crate) fn deform_segments(skeleton: &Skeleton) -> (Vec<(Vec3, Vec3)>, Vec<BoneId>) {
    let world = skeleton.rest_world();
    skeleton
        .ids()
        .filter(|id| skeleton.bone(*id).deform)
        .map(|id| (head_tail(&world[id.index()], skeleton.bone(id).length), id))
        .unzip()
}

/// Méthode de calcul des poids automatiques.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindMethod {
    /// Diffusion de chaleur (qualité, comme Blender / Pinocchio).
    #[default]
    Heat,
    /// Enveloppes par distance aux os (instantané).
    Envelope,
}

impl BindMethod {
    /// Calcule les poids. Si le squelette a des os de visage, la liaison
    /// passe par le pipeline facial ([`face::bind_with_face`]) qui ajoute les
    /// influences localisées du visage ; sinon c'est la liaison du corps.
    pub fn compute(self, mesh: &Mesh, skeleton: &Skeleton) -> Skin {
        if crate::autorig::face::present(skeleton).is_empty() {
            self.compute_body(mesh, skeleton)
        } else {
            face::bind_with_face(mesh, skeleton, self)
        }
    }

    /// Liaison du corps uniquement (sans traitement du visage).
    pub(crate) fn compute_body(self, mesh: &Mesh, skeleton: &Skeleton) -> Skin {
        match self {
            BindMethod::Heat => heat_weights(mesh, skeleton),
            BindMethod::Envelope => Skin::automatic(mesh, skeleton),
        }
    }
}
