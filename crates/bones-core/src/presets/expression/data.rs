//! Donnée d'une expression faciale : un delta de pose (par rapport au repos)
//! pour quelques os du visage, désignés par leur nom canonique (contrat
//! [`crate::autorig::face`]). La donnée est indépendante du squelette : elle
//! s'applique à tout modèle possédant ces os (les absents sont ignorés).

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::Transform;

/// Expression faciale : liste `(nom d'os du visage, delta local sur le repos)`.
/// Seuls des os du visage y figurent ; le corps n'est jamais concerné.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExpressionData {
    pub bones: Vec<(String, Transform)>,
}

impl ExpressionData {
    /// Delta local d'un os du visage (par nom), s'il est posé.
    pub fn delta(&self, name: &str) -> Option<Transform> {
        self.bones.iter().find(|(n, _)| n == name).map(|(_, t)| *t)
    }
}

/// Compte rendu de l'application d'une expression (affiché par l'interface).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExpressionReport {
    /// Os de l'expression présents sur le modèle (effectivement posés).
    pub applied: Vec<String>,
    /// Os de l'expression absents du modèle (ignorés sans erreur).
    pub skipped: Vec<String>,
}

/// Delta mis à l'échelle par `intensity` : interpolation depuis le repos
/// (identité). `intensity` ∈ [0, 1] atténue, > 1 exagère. Jamais de NaN.
pub(crate) fn scaled(delta: &Transform, intensity: f32) -> Transform {
    let k = intensity.clamp(0.0, 4.0);
    Transform {
        translation: delta.translation * k,
        // `slerp` depuis l'identité : moitié d'angle à k = 0,5, le double à k = 2.
        rotation: Quat::IDENTITY.slerp(delta.rotation, k).normalize(),
        scale: Vec3::ONE.lerp(delta.scale, k),
    }
}

/// Compose un delta par-dessus un autre (overlay additif), dans l'espace
/// local de l'os (après le repos) : `base` d'abord, puis `delta`.
pub(crate) fn compose(base: &Transform, delta: &Transform) -> Transform {
    base.mul(delta)
}
