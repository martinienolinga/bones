//! Objets de la scène : accessoires et pièces séparées autour du personnage
//! (épée dans la main, chapeau sur la tête, lunettes, vêtements, parties du
//! corps séparées…), à la manière des objets de Blender.
//!
//! Le maillage principal reste le **corps** (`Rig::mesh` / `Rig::skin`) :
//! auto-rig, peinture de poids et préréglages le ciblent toujours. Les
//! objets supplémentaires vivent dans `Rig::objects()` ; l'interface les
//! liste avec le corps grâce à [`ObjectRef`].
//!
//! # Modèle
//!
//! - [`SceneObject`] : identifiant stable [`ObjectId`] (sérialisé, unique),
//!   nom, maillage partagé par `Arc`, parent, transformation locale,
//!   liaison, visibilité et verrou.
//! - [`Parent`] : le monde, un autre objet, ou un os. Avec un os, la
//!   transformation locale est relative à la transformation monde **posée**
//!   de l'os (parentage d'os de Blender) : l'objet suit l'os quand il bouge.
//!   Les cycles sont refusés ([`ObjectError::Cycle`]).
//! - [`Binding`] : `Rigid` (l'objet suit rigidement son parent) ou
//!   `Skinned` (déformé par le squelette avec ses propres poids). Un objet
//!   déformé est d'abord placé par sa transformation évaluée **au repos**
//!   (placement de liaison), puis déformé par les mêmes matrices de skinning
//!   que le corps (`Editor::skinning_matrices`).
//! - Les poids d'un objet déformé sont calculés en tâche de fond par la même
//!   file que ceux du corps (`Editor::bind_job` / `Editor::finish_bind`) :
//!   transfert depuis le corps ([`SkinMethod::Transfer`], « Data Transfer »
//!   de Blender), diffusion de chaleur, enveloppes, ou poids importés
//!   ([`SkinMethod::Manual`], jamais recalculés).
//! - Les transformations des objets ne sont pas animées (extension
//!   possible) : un accessoire s'anime en suivant l'os auquel il est attaché.
//!
//! # API pour le rendu (phase 2)
//!
//! `Editor::object_draws()` retourne un [`ObjectDraw`] par objet visible :
//! maillage `Arc` + révision (clé des tampons GPU, avec l'identifiant),
//! matrice `model`, et pour un objet déformé ses poids `Arc` + révision. Un
//! objet rigide se dessine avec `model` seule (matrices d'os ignorées) ; un
//! objet déformé applique d'abord `model` (placement de liaison) puis le
//! skinning avec la palette du corps. Les sommets sans poids gardent leur
//! position (comme le corps). `Editor::body_visible()` indique s'il faut
//! dessiner le corps.
//!
//! # API pour l'interface (phase 3)
//!
//! - Liste : `Editor::outliner()` ([`OutlinerRow`] : corps puis objets en
//!   profondeur d'abord), `Editor::object_info(id)`.
//! - Sélection : `Editor::selected_object`, `select_object`,
//!   `select_object_parent` / `_child` / `_next_sibling`, et en mode
//!   `Mode::Object` toucher un objet le sélectionne, le glisser le déplace
//!   dans le plan de l'écran, le manipulateur 3D le transforme (une étape
//!   d'annulation par geste).
//! - Création : `add_primitive(Primitive)`, `add_object_mesh`,
//!   `add_imported_objects` (fichier déjà lu par `io::import_scene`),
//!   `duplicate_object`.
//! - Édition : `delete_object`, `rename_object`, `set_object_visible`,
//!   `set_object_locked`, `set_object_parent` (avec « Keep Transform »),
//!   `parent_object_to_selected_bone`, `clear_object_parent`,
//!   `set_object_binding`, `rebind_object`, `set_object_transform`,
//!   `reorder_object`. Toutes passent par l'historique et retournent une
//!   erreur lisible (en anglais) si l'opération est refusée.

mod bind;
mod draw;
mod hierarchy;
mod import;
mod object;
mod ops;
pub mod pick;
mod remap;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_bind;

pub use bind::BindStamp;
pub(crate) use bind::ObjectBindInput;
pub use draw::{ObjectDraw, OutlinerRow, SkinDraw};
pub use object::{Binding, BodyProps, SceneObject, SkinBinding, SkinMethod};
pub(crate) use remap::ObjectAnchors;

use serde::{Deserialize, Serialize};

use crate::BoneId;

/// Identifiant stable d'un objet (unique dans un document, sérialisé).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub u32);

/// Élément de la scène tel que listé par l'interface : le corps (maillage
/// principal) ou un objet supplémentaire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectRef {
    Body,
    Extra(ObjectId),
}

/// Parent d'un objet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Parent {
    #[default]
    World,
    Object(ObjectId),
    /// Os : transformation locale relative à l'os posé.
    Bone(BoneId),
}

/// Opération refusée sur un objet. Messages destinés à l'utilisateur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectError {
    NotFound,
    /// Le parent demandé est l'objet lui-même ou l'un de ses descendants.
    Cycle,
    NoSuchBone,
    /// Déformation demandée sans squelette.
    NoBones,
    /// Opération impossible sur le corps (suppression, parent…).
    Body,
    /// Objet verrouillé.
    Locked,
    /// Poids fournis qui ne correspondent pas au maillage.
    InvalidWeights,
}

impl std::fmt::Display for ObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ObjectError::NotFound => "Object not found",
            ObjectError::Cycle => "An object cannot be parented to itself or its children",
            ObjectError::NoSuchBone => "Bone not found",
            ObjectError::NoBones => "Add bones before binding an object to the skeleton",
            ObjectError::Body => "Not possible on the body",
            ObjectError::Locked => "Object is locked",
            ObjectError::InvalidWeights => "Weights do not match the object's mesh",
        })
    }
}

impl std::error::Error for ObjectError {}

/// Nom libre parmi `taken` : `base`, puis `base.001`, `base.002`… (un
/// numéro de copie déjà présent est d'abord retiré).
pub(crate) fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    let base = if base.trim().is_empty() {
        "Object"
    } else {
        base.trim()
    };
    if !taken(base) {
        return base.to_owned();
    }
    let stem = match base.rsplit_once('.') {
        Some((stem, n)) if n.len() == 3 && n.bytes().all(|b| b.is_ascii_digit()) => stem,
        _ => base,
    };
    (1..)
        .map(|i| format!("{stem}.{i:03}"))
        .find(|n| !taken(n))
        .expect("itérateur infini")
}
