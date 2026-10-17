//! Préréglages d'animation 3D (humains, animaux, végétaux, créatures) :
//! bibliothèque procédurale intégrée, préréglages utilisateur et importés,
//! détection du type de squelette et des rôles, reciblage, application et
//! création, et fichiers d'échange (format natif `.kb3da`, BVH, glTF, FBX).
//!
//! # API de la galerie (pour l'interface)
//!
//! - **Lister** : [`builtin`] (intégrés, en mémoire, `&'static`) et
//!   [`user::list`] (fichiers `.kb3da` d'un dossier, avec leur fiche). Trier
//!   par [`Category`] / [`Section`] ([`Section::contains`]) et filtrer par
//!   texte ([`PresetInfo::matches`]).
//! - **Aperçu animé** (vignette) : [`Preview::new`] puis [`Preview::segments`]
//!   à l'instant voulu, sans toucher au projet.
//! - **Appliquer** : [`crate::Editor::apply_preset`] avec des [`ApplyOptions`]
//!   (une seule étape d'annulation), [`crate::Editor::preset_roles`] pour la
//!   correspondance des os.
//! - **Créer** depuis l'animation du projet : [`crate::Editor::create_preset`]
//!   avec des [`CreateOptions`].
//! - **Enregistrer / gérer** les préréglages utilisateur : [`user::save`],
//!   [`user::load`], [`user::rename`], [`user::duplicate`], [`user::delete`].
//! - **Importer** un fichier (natif ou BVH/glTF/FBX) : [`import_file`] ;
//!   **exporter** un préréglage (`.kb3da`, BVH, glTF) : [`export_file`].

pub mod apply;
pub mod create;
pub mod error;
pub mod exchange;
pub mod expression;
pub mod file;
#[cfg(test)]
mod io_tests;
pub mod library;
pub mod model;
pub mod params;
pub mod preview;
pub mod procedural;
pub mod reference;
pub mod retarget;
pub mod roles;
#[cfg(test)]
mod sheet;
#[cfg(test)]
mod tests;
pub mod user;

pub use apply::{ApplyMode, ApplyOptions, ApplyReport};
pub use create::CreateOptions;
pub use error::PresetError;
pub use exchange::{export_file, import_bytes, import_file};
pub use expression::{ExpressionData, ExpressionReport};
pub use file::{EXTENSION, load, read_info, save};
pub use library::{builtin, find_builtin};
pub use model::{Category, KeyframeData, Preset, PresetInfo, PresetKind, PresetSource, Procedural, Section};
pub use params::{ParamSpec, Params};
pub use preview::Preview;
pub use procedural::Generator;
pub use reference::RefRig;
pub use roles::{BodyFrame, Chain, Lr, RigType, Role, RoleMap, detect};
