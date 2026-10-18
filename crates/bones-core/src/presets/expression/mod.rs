//! Expressions faciales : bibliothèque intégrée de poses uniques du visage
//! (émotions, expressions courantes, visèmes, animaux), appliquées aux seuls
//! os du visage (contrat [`crate::autorig::face`]) avec intensité, miroir,
//! superposition additive et écriture éventuelle d'une clé.
//!
//! # API pour l'interface
//!
//! - **Lister** : les expressions figurent dans [`crate::presets::builtin`]
//!   sous [`Category::FacialExpression`] ; filtrer par catégorie / section
//!   (groupes « Emotions », « Common », « Visemes », « Animals ») comme les
//!   autres préréglages.
//! - **Appliquer** : [`crate::Editor::apply_expression`] (intensité, miroir
//!   G↔D, additif, « apply as key ») — une seule étape d'annulation, le corps
//!   n'est jamais touché.
//! - **Aperçu** (vignette) : [`crate::presets::Preview::new`] rend l'expression
//!   sur une tête de référence sans toucher au projet.
//! - **Rigs importés** : [`present_mapped`] fait aussi correspondre les noms
//!   d'os du visage d'autres logiciels ([`mapping::foreign`]).
//!
//! [`Category::FacialExpression`]: crate::presets::Category::FacialExpression

mod actions;
mod apply;
mod builder;
mod data;
mod library;
mod mapping;
#[cfg(test)]
mod tests;

pub use apply::{face_pose, present_mapped, reference_head};
pub use data::{ExpressionData, ExpressionReport};
pub(crate) use library::builtin;
pub use mapping::foreign;
