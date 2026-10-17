//! Rig facial façon Auto-Rig Pro (étape « Face ») : mâchoire, yeux et
//! paupières, sourcils, joues, nez, lèvres, oreilles, et si le modèle en a,
//! dents et langue.
//!
//! Contrat de nommage (les préréglages d'expressions s'y réfèrent) : les os
//! sont enfants de `head.x` sauf indication ([`FaceBone::parent`]), suffixes
//! `.l` / `.r` / `.x`, os le long de leur +Y local.
//! - `jaw.x` : pivot à la charnière, queue vers le menton ; `chin.x` ;
//! - `eye.l/.r` : pivot au centre du globe, vers l'avant (+Z) ;
//! - `eyelid_top/bot.l/.r` : pivot au centre du globe, tournés vers la
//!   paupière ; une rotation autour de leur X local (= X monde au repos)
//!   ferme ou ouvre la paupière ;
//! - os de surface (sourcils, joues, nez, narines, lèvres, commissures) :
//!   tête juste sous la peau, pointant vers l'avant (+Z). Au repos, X local =
//!   X monde, Z local = −Y monde (vers le bas). Les côtés `.r` sont les miroirs
//!   des `.l` (mêmes conventions que Blender : une même rotation locale donne
//!   un mouvement symétrique).
//!
//! Étapes : [`FaceMarkers`] (estimés sur la tête du maillage, modifiables)
//! → [`FaceLayout`] (positions des os) → os ajoutés par [`build`].

mod build;
mod guess;
mod layout;
mod markers;
mod profile;
#[cfg(test)]
mod tests;

pub use build::build;
pub use layout::FaceLayout;
pub use markers::{FaceMarkerKind, FaceMarkers};

use crate::{BoneId, Skeleton};

/// Parent d'un os du visage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaceParent {
    Head,
    Jaw,
    /// Os précédent de la chaîne de la langue.
    Tongue,
}

macro_rules! face_bones {
    ($($variant:ident => $name:literal, $parent:ident;)*) => {
        /// Os du visage, dans l'ordre de création.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum FaceBone {
            $($variant,)*
        }

        impl FaceBone {
            pub const ALL: [FaceBone; face_bones!(@count $($variant)*)] = [$(FaceBone::$variant,)*];

            /// Nom de l'os (contrat).
            pub const fn name(self) -> &'static str {
                match self {
                    $(FaceBone::$variant => $name,)*
                }
            }

            pub const fn parent(self) -> FaceParent {
                match self {
                    $(FaceBone::$variant => FaceParent::$parent,)*
                }
            }
        }
    };
    (@count) => { 0 };
    (@count $head:ident $($tail:ident)*) => { 1 + face_bones!(@count $($tail)*) };
}

face_bones! {
    Jaw => "jaw.x", Head;
    Chin => "chin.x", Jaw;
    Tongue1 => "tongue_01.x", Jaw;
    Tongue2 => "tongue_02.x", Tongue;
    Tongue3 => "tongue_03.x", Tongue;
    TeethTop => "teeth_top.x", Head;
    TeethBottom => "teeth_bot.x", Jaw;
    EyeL => "eye.l", Head;
    EyeR => "eye.r", Head;
    EyelidTopL => "eyelid_top.l", Head;
    EyelidTopR => "eyelid_top.r", Head;
    EyelidBottomL => "eyelid_bot.l", Head;
    EyelidBottomR => "eyelid_bot.r", Head;
    BrowInL => "brow_in.l", Head;
    BrowInR => "brow_in.r", Head;
    BrowMidL => "brow_mid.l", Head;
    BrowMidR => "brow_mid.r", Head;
    BrowOutL => "brow_out.l", Head;
    BrowOutR => "brow_out.r", Head;
    CheekL => "cheek.l", Head;
    CheekR => "cheek.r", Head;
    Nose => "nose.x", Head;
    NostrilL => "nostril.l", Head;
    NostrilR => "nostril.r", Head;
    LipTop => "lip_top.x", Head;
    LipTopL => "lip_top.l", Head;
    LipTopR => "lip_top.r", Head;
    LipBottom => "lip_bot.x", Jaw;
    LipBottomL => "lip_bot.l", Jaw;
    LipBottomR => "lip_bot.r", Jaw;
    LipCornerL => "lip_corner.l", Head;
    LipCornerR => "lip_corner.r", Head;
    EarL => "ear.l", Head;
    EarR => "ear.r", Head;
}

/// Noms de tous les os du visage (contrat), dans l'ordre de création.
pub const NAMES: [&str; FaceBone::ALL.len()] = {
    let mut names = [""; FaceBone::ALL.len()];
    let mut i = 0;
    while i < names.len() {
        names[i] = FaceBone::ALL[i].name();
        i += 1;
    }
    names
};

impl FaceBone {
    pub fn from_name(name: &str) -> Option<FaceBone> {
        FaceBone::ALL.into_iter().find(|b| b.name() == name)
    }

    /// Os symétrique (`.l` ↔ `.r`), ou `None` pour un os central (`.x`).
    pub fn mirrored(self) -> Option<FaceBone> {
        crate::rig::mirror::mirror_name(self.name()).and_then(|n| FaceBone::from_name(&n))
    }

    /// Os de la partie basse du visage, qui suit la mâchoire.
    pub fn is_lower(self) -> bool {
        matches!(self.parent(), FaceParent::Jaw | FaceParent::Tongue)
    }
}

/// Os du visage présents dans un squelette, avec leur identifiant.
pub fn present(skeleton: &Skeleton) -> Vec<(FaceBone, BoneId)> {
    FaceBone::ALL
        .into_iter()
        .filter_map(|b| skeleton.find(b.name()).map(|id| (b, id)))
        .collect()
}
