//! Skinning : poids des sommets (format compact), calcul automatique
//! (enveloppes ou diffusion de chaleur) et déformation.

mod deform;
pub mod envelope;
pub mod heat;
mod sparse;
#[cfg(test)]
mod tests;
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
    pub fn compute(self, mesh: &Mesh, skeleton: &Skeleton) -> Skin {
        match self {
            BindMethod::Heat => heat_weights(mesh, skeleton),
            BindMethod::Envelope => Skin::automatic(mesh, skeleton),
        }
    }
}
