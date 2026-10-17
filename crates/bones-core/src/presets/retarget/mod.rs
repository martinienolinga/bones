//! Reciblage d'une animation d'un squelette source vers un squelette cible.
//!
//! Méthode (squelette intermédiaire par rôles, Monzani et al. 2000 ;
//! transfert de rotations monde relatives au repos alignées, comme l'IK
//! Retargeter d'Unreal avec « auto-align » ou les outils de Rokoko et
//! Blender) :
//! 1. correspondance os à os par rôle (chaînes rééchantillonnées quand le
//!    nombre d'os diffère : 2 os de colonne ↔ 4), puis par nom ;
//! 2. pour chaque os apparié, la rotation monde de la source *relative à
//!    son repos* est exprimée dans le repère du corps (avant/haut/gauche),
//!    reportée dans celui de la cible, puis appliquée à l'os cible
//!    préalablement aligné sur la direction de repos de la source (pose T
//!    contre pose A, axes d'os différents) ;
//! 3. le déplacement du bassin est mis à l'échelle (hauteur des hanches) ;
//! 4. option : IK des pieds (et des pattes) pour reproduire les appuis à
//!    l'échelle de la cible, sans glissement ;
//! 5. les os sans correspondance gardent leur pose.

mod clip;
mod mapping;
mod transfer;

#[cfg(test)]
mod tests;

use glam::Quat;

pub use clip::retarget_clip;
pub use mapping::Mapping;
pub(crate) use transfer::Retargeter;

use crate::BoneId;

/// Réglages du reciblage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RetargetOptions {
    /// Gauche ↔ droite.
    pub mirror: bool,
    /// Aligner les directions de repos (pose T ↔ pose A).
    pub align_rest: bool,
    /// Reporter le déplacement du bassin (sinon il reste en place).
    pub root_motion: bool,
    /// Replacer pieds et pattes par IK aux appuis mis à l'échelle.
    pub foot_ik: bool,
}

impl Default for RetargetOptions {
    fn default() -> Self {
        Self {
            mirror: false,
            align_rest: true,
            root_motion: true,
            foot_ik: true,
        }
    }
}

/// Source d'un os cible : mélange de deux os source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sample {
    pub a: BoneId,
    pub b: BoneId,
    pub w: f32,
    /// Alignement de repos (monde cible), appliqué avant le delta source.
    pub align: Quat,
    /// Reporte aussi la translation (bassin, os au sol).
    pub translate: bool,
}

/// Compte rendu d'un reciblage (affiché par l'interface).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// Os cible animés et leur source (noms).
    pub mapped: Vec<(String, String)>,
    /// Os cible sans correspondance (gardent leur pose).
    pub unmapped: Vec<String>,
    pub warnings: Vec<String>,
}
